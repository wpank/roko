# Prompt: 06-neuro

You are a fresh Claude Opus agent. Zero prior context. Read every file this prompt references before writing.

## Your mission

Generate `/Users/will/dev/nunchi/roko/roko/docs/06-neuro/`. This is the Neuro cognitive cross-cut — the knowledge store formerly known as Grimoire. Covers 6 knowledge types, 4 validation tiers × type base half-life decay, HDC encoding (10,240-bit BSC), cross-domain transfer via structural analogy, Ebbinghaus × tier, knowledge backup/restore (replaces succession), Library of Babel, NeuroStore API.

## Step 1 — Context pack (MANDATORY)

Read all 7 files in `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/` in order.

## Step 2 — refactoring-prd canonical sources

1. `/Users/will/dev/nunchi/roko/refactoring-prd/03-cognitive-subsystems.md` §1 Neuro — all sections
2. `/Users/will/dev/nunchi/roko/refactoring-prd/04-knowledge-and-mesh.md` §1 Knowledge Architecture, §5 Knowledge Backup & Restore (4-step lifecycle)
3. `/Users/will/dev/nunchi/roko/refactoring-prd/01-synapse-architecture.md` §Decay enum (Ebbinghaus variant, memory management not mortality)
4. `/Users/will/dev/nunchi/roko/refactoring-prd/09-innovations.md` §III Somatic Landscape (integration with Neuro), §XIII Cross-Domain Insight Resonance (false-positive math, threshold 0.526)
5. `/Users/will/dev/nunchi/roko/refactoring-prd/08-translation-guide.md`
6. `/Users/will/dev/nunchi/roko/refactoring-prd/07-implementation-priorities.md` §Tier 2A/2B (NeuroStore wiring, 4-tier distillation)

## Step 3 — SOURCE-INDEX entry `## 06-neuro.md`

Read every file. Key legacy:
- `bardo-backup/prd/04-memory/00-overview.md`, `01-grimoire.md`, `01b-grimoire-memetic.md`, `01c-grimoire-hdc.md`, `02-emotional-memory.md`, `06-economy.md`, `09-safety.md`, `13-library-of-babel.md`
- `bardo-backup/prd/shared/hdc-vsa.md`, `hdc-applications.md`, `hdc-fingerprints.md`
- `bardo-backup/tmp/mori-refactor/09-memory-and-knowledge.md`, `12-cognitive-architecture.md`
- `bardo-backup/tmp/agent-chain/04-hdc.md` — HDC math from first principles
- `bardo-backup/tmp/agent-chain/05-knowledge-layer.md` — 6 knowledge types origin
- `bardo-backup/tmp/agent-chain/15-dynamic-context-assembly.md`
- `bardo-backup/tmp/death/tools/c01-retrieval.md`, `c02-structure.md`, `c03-token-economics.md`, `c04-pre-computation.md` (extract non-mortality content)

## Step 4 — implementation-plans

- `12a-cognitive-layer.md` §D Knowledge & Memory (D1–D18: **4-tier distillation pipeline**, knowledge types + storage, HDC integration with half-lives **Insight 30d / Heuristic 90d / Fact 365d / Warning 7d / CausalLink 60d / StrategyFragment 14d / AntiKnowledge never (floor 0.3)**). §R1 (roko-neuro crate creation). D1-D6 is the distillation pipeline. D7-D11 is types + storage + AntiKnowledge + query API. D12-D18 is HDC wiring.

## Step 5 — active code

- Glob `/Users/will/dev/nunchi/roko/roko/crates/roko-neuro/src/**/*.rs` (if exists, else it's scaffold)
- Read `/Users/will/dev/nunchi/roko/roko/crates/bardo-primitives/src/hdc.rs` — HDC primitives (to rename roko-primitives)
- Read `/Users/will/dev/nunchi/roko/roko/crates/roko-index/src/hdc.rs`
- Read `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/hdc_clustering.rs` (K-medoids)
- Read `/Users/will/dev/nunchi/roko/roko/crates/roko-golem/src/grimoire.rs` (scaffold to delete after dissolution)

## Step 6 — Output and sub-doc plan

```bash
mkdir -p /Users/will/dev/nunchi/roko/roko/docs/06-neuro
```

Write **16 sub-docs** plus `INDEX.md`:

| # | Filename | Content |
|---|---|---|
| 00 | `00-vision-and-grimoire-rename.md` | Neuro subsystem vision. Semantic wrapper around Substrate. NOT a replacement for Substrate — uses put/query underneath but adds knowledge-specific logic. Rename from Grimoire. Why "Neuro" (neuroscience metaphor). Persistent, tiered, HDC-indexed knowledge. |
| 01 | `01-six-knowledge-types.md` | Insight, Heuristic, Warning, CausalLink, StrategyFragment, AntiKnowledge. For each: definition, Rust type, coding example, chain example, use case. Full Kind enum. |
| 02 | `02-four-validation-tiers.md` | Transient (0.1×), Working (0.5×), Consolidated (1.0×), Persistent (5.0×). Per-tier multiplier on type base half-life. Promotion/demotion criteria. Automatic based on outcome feedback. |
| 03 | `03-type-half-lives.md` | Base half-lives per type: Insight 30d, Heuristic 90d, Fact 365d, Warning 7d, CausalLink 60d, StrategyFragment 14d, AntiKnowledge never (floor 0.3). Rationale for each value. Example: Persistent Insight = 5.0 × 30 = 150 days. |
| 04 | `04-hdc-vsa-foundations.md` | Hyperdimensional Computing / Vector Symbolic Architectures. 10,240-bit Binary Spatter Code (BSC) vectors. Johnson-Lindenstrauss 1984 (need ≥4604 dim for N=100K, ε=0.1; 10,240 is generous). Kanerva 2009 (Cognitive Computation 1(2)). Neubert 2022 (Proc IEEE, VSA survey). Kleyko 2022 (ACM Computing Surveys 55(6)). Plate 1994. Frady 2021 (resonator networks). |
| 05 | `05-hdc-operations.md` | XOR binding. Majority bundling. Cyclic-shift permutation. Hamming similarity (XOR + popcount). Performance: ~13ns per comparison with AVX-512, ~170µs for 100K entries. ARM NEON ~2-3× slower. |
| 06 | `06-hdc-knowledge-encoding.md` | Text content → 10,240-bit BSC vector. Structured encoding (BIND for association, BUNDLE for merging, PERM for sequence). Wrapping `bardo-primitives::HdcVector` for knowledge text. Three-tier similarity search: Bloom filter (fast reject) → approximate (coarse) → exact top-K. |
| 07 | `07-ebbinghaus-decay-with-tier.md` | Ebbinghaus 1885 forgetting curve. Formula: `weight = exp(-age / (strength × scale_ms))`. `effective_decay = tier_multiplier × type_base_half_life`. Successful use increases strength → decay slows. Failed use decreases strength → decay accelerates. Tier progression emerges naturally. Integration with Neuro. |
| 08 | `08-cross-domain-hdc-transfer.md` | Structural analogy across domains. `BIND(high_complexity, more_review)` (coding) ≈ `BIND(high_volatility, more_caution)` (chain). High Hamming similarity because both encode `BIND(high_uncertainty, more_verification)`. When coding agent learns, chain agent benefits. Cross-domain insights humans wouldn't consider — HDC space reveals structural similarities invisible at surface. |
| 09 | `09-false-positive-math.md` | Threshold selection for cross-domain resonance. For independent random 10,240-bit vectors, Hamming similarity ~ Normal(0.5, σ²) with σ = 1/(2√n) = 0.00494. Threshold table: 0.512 (Z=2.43, <1% false positive single pair), **0.526 (Z=5.26, <1% against 100K vocabulary, Bonferroni corrected)**, 0.54 (Z=8.10, <10⁻¹⁵). Recommendation: threshold 0.526. Additional validation: confirm by 2 independent agents (quadratic reduction of false positives). |
| 10 | `10-knowledge-query-api.md` | NeuroStore trait: put, query, decay, gc. Query parameters (semantic similarity + temporal relevance + affect filters). Hamming-nearest-neighbor search. Integration with Composer for context assembly. |
| 11 | `11-antiknowledge-challenge.md` | AntiKnowledge type — things that seem true but aren't. Challenge mechanism (contradicts existing knowledge). On-chain: 2× stake requirement. Locally: "this insight was wrong" with evidence. Reduces confidence of contradicted entries. Floor 0.3 (never fully forgotten). |
| 12 | `12-4-tier-distillation-pipeline.md` | From 12a-cognitive-layer.md D1-D6: Raw Episodes → Insights (pattern detection across episodes: "when X happened, Y consistently followed") → Heuristics (3+ confirmations → actionable rule) → PLAYBOOK (top heuristics → PLAYBOOK.md). Confirmation boost: independent validation extends weight ×1.5. Temporal decay per type. Full algorithm. |
| 13 | `13-somatic-integration.md` | Somatic landscape (from topic 09-daimon) reads from Neuro. Mood-congruent retrieval. 15% mandatory contrarian retrieval (Bower 1981) prevents echo chambers. Cross-reference. |
| 14 | `14-library-of-babel.md` | Cross-collective knowledge via public Korai chain. What gets published vs what stays local. Configurable publishing policies. Non-alpha insights, general heuristics, validated patterns, warnings, anti-knowledge. Cross-reference topic 13-coordination and 14-identity-economy. |
| 15 | `15-knowledge-backup-restore.md` | **Replaces succession entirely.** User-controlled 4-step: BACKUP (export NeuroStore: JSONL + HDC vectors + tier metadata + provenance) → DELETE (user explicitly deletes agent) → CREATE (fresh NeuroStore) → RESTORE (selective import: pick entries, start at Transient tier, provenance tracks origin "restored from agent X on date Y"). No biological metaphor. No automatic transfer. |
| 16 | `16-current-status-and-gaps.md` | roko-neuro crate scaffold. bardo-primitives HDC exists (to rename roko-primitives). roko-index HDC exists. roko-learn hdc_clustering exists. Wiring gaps from 12a §D (D1-D6 distillation missing, D12-D18 HDC wiring missing). R1 crate creation plan. What knowledge types need implementing. |

Plus `INDEX.md`.

## Step 7-9 — Rules, INDEX, self-check

Per context-pack rules. ≥200 lines per sub-doc, ≥4500 total. Citations: Kanerva 2009, Neubert 2022, Kleyko 2022, Plate 1994, Frady 2021, Ebbinghaus 1885, Johnson-Lindenstrauss 1984, Bower 1981, Damasio 1994, Park 2023 (arXiv:2304.03442), Sumers 2023 (arXiv:2309.02427), Mattar-Daw. **Minimum 14+ HDC/VSA references.**

Cross-reference topics 00-architecture (Decay enum, Engram), 03-composition (knowledge injected into context), 05-learning (distillation pipeline), 08-chain (HDC on-chain precompile), 09-daimon (somatic landscape), 13-coordination (pheromones), 14-identity-economy (Library of Babel).

## CRITICAL REMINDERS

- DO NOT SUMMARIZE. DO NOT TRUNCATE. PRESERVE ALL 14+ HDC CITATIONS.
- Neuro is a **semantic wrapper around Substrate**, not a replacement. Make this very clear.
- Apply rename: grimoire → neuro. Clade → collective/mesh.
- Knowledge backup/restore is 4-step and **replaces succession entirely**. No biological framing.
- No death / mortality / thanatopsis (Neuro has Ebbinghaus decay but that's memory management, NOT mortality).
- Use Write tool. Don't ask questions. Continue.
