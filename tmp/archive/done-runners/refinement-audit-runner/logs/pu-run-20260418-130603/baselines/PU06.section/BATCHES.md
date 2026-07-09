# Batch Execution Contract

9 batches ordered for unattended execution. The goal is not just to “cover the neuro docs”, but to let an agent turn neuro-parity findings into bounded work that can run overnight without guessing which neuro surfaces are already real.

---

## Batch Posture

- Default strategy: **activate already-shipped neuro runtime surfaces before adding research-grade HDC, transfer, or chain systems**.
- Treat `crates/roko-cli/src/orchestrate.rs` and `crates/roko-neuro/src/context.rs` as the primary production conflict hotspot.
- Treat `knowledge_store.rs`, `tier_progression.rs`, `distiller.rs`, and `crates/roko-cli/src/main.rs` as the main contract modules.
- If a task starts requiring network protocols, token economics, or research-heavy HDC systems, record the seam and stop.
- Every completed batch should leave behind:
  - code changes,
  - verification command output,
  - explicit deferrals,
  - and any newly clarified runtime contract.

Required reads for every batch:

- [00-INDEX.md](00-INDEX.md)
- the owning section file(s) named below
- [SOURCE-INDEX.md](SOURCE-INDEX.md)
- [context-pack/agent-runbook.md](context-pack/agent-runbook.md)
- [context-pack/carry-forward-map.md](context-pack/carry-forward-map.md)

---

## Recommended Serial Order

For a single long-running agent run, prefer:

`N1 -> N2 -> N3 -> N5 -> N6 -> N4 -> N7 -> N8 -> N9`

This order first activates the strongest dormant runtime seam, then clarifies the live query contract, then hardens distillation, then handles source ownership and backup surfaces, and only after that resolves the larger transfer, HDC, and meta-doc honesty work.

---

## Batch Overview

| Batch | Tasks | Purpose | Primary Write Scope | Verify Focus | Est. LOC |
|-------|-------|---------|---------------------|--------------|----------|
| N1 | C.07, C.09, C.10 | Activate `ContextAssembler` on a real orchestrator path | `roko-cli`, `roko-neuro`, `roko-compose` | `cargo test -p roko-cli -p roko-neuro -p roko-compose` | 220 |
| N2 | B.15, C.03-C.05, C.08, C.11, C.17-C.19 | Make the neuro query contract explicit and less doc-driven | `roko-neuro`, `roko-cli` | `cargo test -p roko-neuro -p roko-cli` | 220 |
| N3 | D.01-D.03, D.11-D.13 | Harden the real distillation and promotion contract | `roko-neuro`, `roko-dreams`, `roko-cli` | `cargo test -p roko-neuro -p roko-dreams -p roko-cli` | 240 |
| N4 | D.14 | Resolve scheduler / quality-report ambiguity around the distillation pipeline | `roko-neuro`, `roko-learn`, `roko-dreams`, docs if needed | `cargo test -p roko-neuro -p roko-dreams -p roko-cli` | 140 |
| N5 | E.11, E.17-E.18, E.20 | Make neuro ingest/source ownership honest and bounded | `roko-neuro`, `roko-cli`, docs | `cargo test -p roko-neuro -p roko-cli` | 220 |
| N6 | E.15-E.16, E.19, E.22 | Add or explicitly demote neuro backup / restore / publish surfaces | `roko-cli`, `roko-neuro`, `roko-fs`, docs | `cargo test -p roko-cli -p roko-neuro -p roko-fs` | 260 |
| N7 | C.12-C.16, C.20-C.21 | Make doc-08 cross-domain transfer honest and optionally ship one very small seam | docs first, then `roko-neuro` only if scope stays tiny | `rg -n "Resonance|TransferRisk|DomainProfile|ConfirmationTracker|AnalogyResult" crates docs/06-neuro tmp/docs-parity/06` | 120 |
| N8 | B.08-B.14, B.19-B.21 | Decide which advanced HDC enablers are worth building now and defer the rest explicitly | `roko-primitives`, `roko-neuro`, `roko-index`, docs | `cargo test -p roko-primitives -p roko-neuro -p roko-index -p roko-learn` | 240 |
| N9 | A.03, A.09, A.12, A.20, F.05-F.15 | Clean up stale meta-docs, schema drift, and contradictory frontier status claims | docs, `CLAUDE.md`, parity notes | `rg -n "roko-golem|Fact|FACT_HALF_LIFE_DAYS|KnowledgeCrystal|Pheromone|Dreams cycle|cross-domain transfer" docs CLAUDE.md tmp/docs-parity/06` | 100 |

---

## Dependency Graph

| Batch | Depends on |
|-------|------------|
| N1 | — |
| N2 | N1 |
| N3 | — |
| N4 | N3 |
| N5 | — |
| N6 | N5 |
| N7 | N2 |
| N8 | N2 |
| N9 | N4, N6, N7, N8 |

Why `N2 -> N1`:

- query and threshold hardening is easier once the real runtime retrieval path is the one being hardened.

Why `N4 -> N3`:

- scheduler and quality-report decisions depend on first being honest about the real D1/D2/D3 contract.

Why `N6 -> N5`:

- backup and restore contracts are easier to define once source ownership and ingest staging are clearer.

Why `N7 -> N2` and `N8 -> N2`:

- cross-domain and advanced-HDC decisions should build on a clarified live query contract, not on stale assumptions.

Why `N9` comes last:

- the doc cleanup should reflect the runtime decisions made by the earlier batches, not race ahead of them.

Parallel-safe groups:

- `{N1, N3, N5}` can start immediately.
- `N2` waits for `N1`.
- `N4` waits for `N3`.
- `N6` waits for `N5`.
- `N7` and `N8` wait for `N2`.
- `N9` should be last.

Conflict groups:

| Group | Crates / Files | Batches |
|-------|----------------|---------|
| orchestrate-neuro | `crates/roko-cli/src/orchestrate.rs` | N1, N2, N3, N5 |
| neuro-query | `crates/roko-neuro/src/knowledge_store.rs`, `context.rs`, `lib.rs` | N1, N2, N5, N7, N8 |
| distillation | `crates/roko-neuro/src/distiller.rs`, `tier_progression.rs`, `crates/roko-dreams/src/cycle.rs` | N3, N4 |
| backup-cli | `crates/roko-cli/src/main.rs`, neuro backup helpers, layout files | N6 |
| advanced-hdc | `crates/roko-primitives`, `crates/roko-index`, `crates/roko-neuro/src/hdc.rs` | N8 |
| docs-contract | `docs/06-neuro/*`, `CLAUDE.md`, `tmp/docs-parity/06/*` | N7, N9 |

---

## Batch Details

### N1 — ContextAssembler Production Activation

**Owns**: `C.07`, `C.09`, `C.10`

**Read first**:

- [C-query-crossdomain-context.md](C-query-crossdomain-context.md)
- [E-somatic-exchange-backup.md](E-somatic-exchange-backup.md)

**Problem**: the richest retrieval pipeline in the batch already exists, but orchestrator production paths still bypass it and go directly to `KnowledgeStore::query(...)` / `query_kind(...)`.

**Scope**:

1. Identify one real orchestrator path where `ContextAssembler::gather(...)` should own retrieval.
2. Thread the smallest required inputs for budget, PAD, and somatic bias through that path.
3. Keep fallback behavior deterministic if some metadata is unavailable.
4. Add tests or runtime evidence that the assembler is no longer library-only.

**Out of scope**:

- redesigning prompt composition,
- rewriting every neuro query callsite in one batch,
- implementing doc-08 cross-domain transfer,
- changing the assembler algorithm itself unless activation requires a tiny fix.

**Files**:

- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-neuro/src/context.rs`
- `crates/roko-compose` only if downstream context wiring needs small adjustments

**Verify**:

```bash
cargo test -p roko-cli -p roko-neuro -p roko-compose
rg -n "ContextAssembler::new|\\.gather\\(|query_kind\\(|query\\(" crates/roko-cli crates/roko-neuro crates/roko-compose
```

**Acceptance criteria**:

- at least one production caller constructs `ContextAssembler`,
- PAD / contrarian / budget logic runs on a real path,
- later agents do not need to infer why the best retrieval pipeline is still dormant.

---

### N2 — Query Contract And Threshold Hardening

**Owns**: `B.15`, `C.03-C.05`, `C.08`, `C.11`, `C.17-C.19`

**Read first**:

- [B-hdc-foundations-operations.md](B-hdc-foundations-operations.md)
- [C-query-crossdomain-context.md](C-query-crossdomain-context.md)

**Problem**: the live neuro query behavior is real, but thresholds, scoring, stats shapes, and config semantics are still spread across docs, private constants, and implicit defaults.

**Scope**:

1. Decide whether a similarity threshold or `min_similarity` contract should exist in code now.
2. Make the current query-score formula and stats/confirmation structs easier to treat as canonical.
3. Add a small configuration surface only if it simplifies the runtime contract.
4. Prefer honest code/docs alignment over speculative “future-proof” APIs.

**Out of scope**:

- full analogy / resonance / confirmation protocol work,
- `ItemMemory` or `ResonatorNetwork`,
- large search-architecture changes.

**Files**:

- `crates/roko-neuro/src/knowledge_store.rs`
- `crates/roko-neuro/src/lib.rs`
- `crates/roko-neuro/src/context.rs`
- `crates/roko-cli/src/orchestrate.rs`

**Verify**:

```bash
cargo test -p roko-neuro -p roko-cli
rg -n "CONFIRMATION_BOOST|min_similarity|KnowledgeStats|KnowledgeConfirmationRecord|DEFAULT_MIN_SUPPORT|ContextAssemblerConfig|search_by_tier|search_hdc" crates/roko-neuro crates/roko-cli
```

**Acceptance criteria**:

- one explicit query contract exists for thresholds and scoring,
- struct-shape drift is reduced or explicitly documented,
- later batches do not have to guess which neuro query semantics are canonical.

---

### N3 — Distillation Contract Hardening

**Owns**: `D.01-D.03`, `D.11-D.13`

**Read first**:

- [D-distillation-progression.md](D-distillation-progression.md)
- [A-knowledge-types-tiers-decay.md](A-knowledge-types-tiers-decay.md)

**Problem**: D1 and Dream-cycle progression are real, but warning extraction, promotion guards, and the playbook-output story are still split between runtime and spec.

**Scope**:

1. Decide which deterministic checks belong in the current distillation path now.
2. If `extract_warnings` and promotion guards are too large, leave explicit runtime boundaries instead of vague “missing” notes.
3. Make the D3 playbook-output contract more obvious, even if the result is a docs correction rather than a new writer.
4. Preserve the existing `5+ @ >= 0.7` promotion core unless there is a clear bug.

**Out of scope**:

- a research-heavy clustering rewrite,
- inventing a second playbook artifact hierarchy,
- Dreams architecture redesign.

**Files**:

- `crates/roko-neuro/src/distiller.rs`
- `crates/roko-neuro/src/tier_progression.rs`
- `crates/roko-dreams/src/cycle.rs`
- `crates/roko-cli/src/orchestrate.rs`

**Verify**:

```bash
cargo test -p roko-neuro -p roko-dreams -p roko-cli
rg -n "spawn_episode_distillation|write_playbook|extract_warnings|cross_validation|anti_knowledge|TierProgressionDecision" crates/roko-neuro crates/roko-dreams crates/roko-cli
```

**Acceptance criteria**:

- the current D1/D2/D3 contract is understandable from code and docs,
- promotion checks are either real or explicitly out of scope,
- later agents do not have to reverse-engineer whether the playbook path is live.

---

### N4 — Distillation Scheduling And Quality Boundary

**Owns**: `D.14`

**Read first**:

- [D-distillation-progression.md](D-distillation-progression.md)

**Problem**: the docs describe a substantial scheduler and quality-reporting layer that does not exist, while runtime cadence is currently split across episode hooks and Dream cycles.

**Scope**:

1. Decide whether to build a small scheduling surface now or explicitly map the real cadence and demote the rest.
2. If a scheduler is added, keep it minimal and aligned with existing hooks.
3. If quality reporting is added, prefer one concrete report over a large metrics taxonomy.

**Out of scope**:

- a full distillation-ops platform,
- large new telemetry requirements,
- batch-budget optimization research.

**Files**:

- `crates/roko-neuro/src/distiller.rs`
- `crates/roko-dreams/src/cycle.rs`
- `crates/roko-learn` only if existing cadence helpers are reused
- docs if runtime is intentionally kept simple

**Verify**:

```bash
cargo test -p roko-neuro -p roko-dreams -p roko-cli
rg -n "DistillationScheduler|DistillationQualityReport|distiller_every_n|spawn_episode_distillation|run_budgeted" crates
```

**Acceptance criteria**:

- there is one honest answer for “when does distillation run?”,
- scheduler and quality-report claims are either implemented or explicitly demoted,
- later batches do not inherit a fake operations layer.

---

### N5 — Ingest Source Ownership And Safety Contract

**Owns**: `E.11`, `E.17-E.18`, `E.20`

**Read first**:

- [E-somatic-exchange-backup.md](E-somatic-exchange-backup.md)
- [A-knowledge-types-tiers-decay.md](A-knowledge-types-tiers-decay.md)

**Problem**: the docs describe multiple inflow channels and a staged safety pipeline, but the live runtime mostly has direct ingest with self-distilled provenance and no staging semantics.

**Scope**:

1. Decide what source ownership exists in runtime now and make it explicit.
2. Add a bounded source/staging contract only if it helps a real ingest path immediately.
3. Keep emotional/arousal drift corrections small and tied to real code, not doc ideals.
4. Leave clear handoffs for mesh, Korai, Lethe, and token-governed ingest.

**Out of scope**:

- network replication,
- challenge markets or token economics,
- large quarantine/sandbox infrastructure.

**Files**:

- `crates/roko-neuro/src/lib.rs`
- `crates/roko-neuro/src/knowledge_store.rs`
- `crates/roko-cli/src/orchestrate.rs`
- docs for source-channel and safety demotion if needed

**Verify**:

```bash
cargo test -p roko-neuro -p roko-cli
rg -n "source:|KnowledgeSource|ingest\\(|quarantine|sandbox|consensus|adopt|default_half_life_hours|arousal" crates/roko-neuro crates/roko-cli
```

**Acceptance criteria**:

- later agents can name the real ingest sources without guessing,
- safety staging is either bounded and real or clearly absent,
- docs stop implying five live inflow channels when runtime is narrower.

---

### N6 — Backup / Restore / Publish Surface

**Owns**: `E.15-E.16`, `E.19`, `E.22`

**Read first**:

- [E-somatic-exchange-backup.md](E-somatic-exchange-backup.md)
- [SOURCE-INDEX.md](SOURCE-INDEX.md)

**Problem**: docs `14` and `15` describe backup, restore, publish, manifest, and lineage semantics, but the CLI today only exposes `Query`, `Stats`, and `Gc`.

**Scope**:

1. Decide whether batch `06` should ship a local backup/restore MVP or explicitly keep doc `15` as design-only.
2. If implementing, start with local filesystem backup and manifest integrity, not network publication.
3. Keep lineage discount and publishing policies subordinate to the basic backup contract.
4. Leave exact file layout and verification evidence for unattended restore runs.

**Out of scope**:

- Korai publication,
- token billing,
- distributed sync,
- restore-time trust markets.

**Files**:

- `crates/roko-cli/src/main.rs`
- `crates/roko-neuro`
- `crates/roko-fs` if layout helpers are useful
- docs `14` and `15`

**Verify**:

```bash
cargo test -p roko-cli -p roko-neuro -p roko-fs
rg -n "enum NeuroCmd|Backup|Restore|Publish|BackupManifest|manifest.json|knowledge.jsonl" crates/roko-cli crates/roko-neuro crates/roko-fs
```

**Acceptance criteria**:

- there is one obvious answer for whether neuro backup/restore is live,
- if live, it has a clear manifest/layout contract,
- if not live, the docs no longer read like the CLI already supports it.

---

### N7 — Cross-Domain Transfer Boundary

**Owns**: `C.12-C.16`, `C.20-C.21`

**Read first**:

- [C-query-crossdomain-context.md](C-query-crossdomain-context.md)
- [F-status-frontier.md](F-status-frontier.md)

**Problem**: doc `08` opens as if cross-domain transfer is implemented, but the large resonance / risk / confirmation block is still grep-negative in production code.

**Scope**:

1. Make doc `08` honest first.
2. Only ship a code change if there is a very small, obviously useful seam such as a transfer-readiness helper or thresholded similarity report.
3. Keep Dreams-side cross-domain hypotheses clearly distinguished from doc-08 resonance transfer.

**Out of scope**:

- multi-agent confirmation protocols,
- full domain-distance models,
- analogical query engines that need `ItemMemory`,
- implementing the entire normative Rust block from the doc.

**Files**:

- docs `06-neuro/08-*`
- `tmp/docs-parity/06/*`
- `crates/roko-neuro` only if a tiny real seam is chosen

**Verify**:

```bash
rg -n "Resonance|TransferRisk|TransferRecommendation|DomainProfile|DomainDistance|AnalogyResult|ConfirmationTracker" crates docs/06-neuro tmp/docs-parity/06
```

**Acceptance criteria**:

- doc `08` no longer overclaims implementation status,
- later agents can tell exactly what is real versus merely adjacent,
- any shipped seam is small, testable, and clearly not the full transfer system.

---

### N8 — Advanced HDC Enabler Triage

**Owns**: `B.08-B.14`, `B.19-B.21`

**Read first**:

- [B-hdc-foundations-operations.md](B-hdc-foundations-operations.md)
- [C-query-crossdomain-context.md](C-query-crossdomain-context.md)

**Problem**: the docs present several HDC extensions as if they are near-term engineering work, but many are really research or later-performance prerequisites.

**Scope**:

1. Decide whether any small enabler such as `BundleAccumulator`, `ItemMemory`, or a structured query helper is worth shipping now.
2. Defer `ResonatorNetwork`, SIMD kernels, three-tier search, ontology schema, and episode-compression helpers unless there is a concrete runtime need.
3. Make the prerequisite story for future cross-domain work explicit.

**Out of scope**:

- performance-driven unsafe-code work,
- full ontology systems,
- speculative search stacks with no caller,
- HDC research for its own sake.

**Files**:

- `crates/roko-primitives/src/hdc.rs`
- `crates/roko-neuro/src/hdc.rs`
- `crates/roko-index`
- docs `04`, `05`, `06`, `09`

**Verify**:

```bash
cargo test -p roko-primitives -p roko-neuro -p roko-index -p roko-learn
rg -n "BundleAccumulator|ItemMemory|ResonatorNetwork|query_by_role|unbind_role|SIMD|compress_episode|KnowledgeOntology" crates docs/06-neuro tmp/docs-parity/06
```

**Acceptance criteria**:

- later agents know which HDC prerequisites are actually worth pursuing next,
- research-only items are clearly deferred,
- no one needs to infer from prose whether these advanced HDC helpers exist.

---

### N9 — Meta-Docs And Frontier Honesty

**Owns**: `A.03`, `A.09`, `A.12`, `A.20`, `F.05-F.15`

**Read first**:

- [A-knowledge-types-tiers-decay.md](A-knowledge-types-tiers-decay.md)
- [F-status-frontier.md](F-status-frontier.md)

**Problem**: the code-side neuro rename is complete, but schema docs, crate maps, and frontier summaries still contain stale or contradictory claims.

**Scope**:

1. Remove stale `roko-golem` references from meta-docs.
2. Fix `Fact` / `FACT_HALF_LIFE_DAYS` / schema-field drift where docs still describe old neuro shapes.
3. Mark frontier concepts as design-only where needed.
4. Preserve the distinction between “partial but real” and “not implemented”.

**Out of scope**:

- broad documentation rewrites beyond the stale or contradictory parts,
- changing runtime behavior just to fit old docs,
- re-auditing all earlier parity batches.

**Files**:

- `CLAUDE.md`
- `docs/00-architecture/*`
- `docs/06-neuro/*`
- `docs/09-daimon/*`
- `docs/10-dreams/*`
- `tmp/docs-parity/06/*`

**Verify**:

```bash
rg -n "roko-golem|Fact|FACT_HALF_LIFE_DAYS|KnowledgeCrystal|MetabolismMetrics|NeurosymbolicStore|Dreams cycle|Pheromone system|cross-domain transfer" docs CLAUDE.md tmp/docs-parity/06
```

**Acceptance criteria**:

- meta-docs stop contradicting shipped crate reality,
- frontier concepts are labeled honestly,
- later agents can trust the docs to distinguish shipping code from design intent.
