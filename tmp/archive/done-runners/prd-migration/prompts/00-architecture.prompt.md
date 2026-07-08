# Prompt: 00-architecture

You are a fresh Claude Opus agent. You have zero prior context about Roko. This prompt
is your complete briefing. Read every file it references before you start writing.

## Your mission

Generate the `00-architecture/` topic folder for the Roko PRD documentation. This is the
foundational topic — it describes the Synapse Architecture, Engrams, the 6 Synapse traits,
the universal cognitive loop, the 5 layers, cognitive cross-cuts, C-Factor, the crate map,
provenance, autocatalytic improvement, and the design principles. Every other topic in the
documentation builds on the concepts you define here.

Your output goes to `/Users/will/dev/nunchi/roko/roko/docs/00-architecture/`.

## Step 1 — Read the context pack (MANDATORY, in order)

Use the Read tool to read these files, in order, before anything else:

1. `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/00-ALWAYS-READ-FIRST.md`
2. `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/01-naming-map.md`
3. `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/02-reframe-rules.md`
4. `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/03-concepts-lifecycle.md`
5. `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/04-writing-rules.md`
6. `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/05-source-files.md`
7. `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/06-output-structure.md`

These are non-negotiable. Read them fully. They define:
- The naming map (Bardo→Roko, Golem→Agent, Grimoire→Neuro, Styx→Mesh, GNOS→KORAI/DAEJI, Clade→Collective/Mesh — **NOT fleet**, Signal→Engram, "1 noun 6 verbs"→Synapse Architecture)
- The reframe rules (no mortality, no death, no terminal phases, no succession, no stochastic death clocks)
- The writing rules (DO NOT SUMMARIZE, DO NOT TRUNCATE, PRESERVE ALL CITATIONS, write for zero-context readers, use 5-layer taxonomy, integrate Synapse language)
- The output structure (where to write, what INDEX.md must contain, what each sub-doc must contain)

## Step 2 — Read the canonical refactoring-prd spec

Use Read to read every one of these files IN FULL:

1. `/Users/will/dev/nunchi/roko/refactoring-prd/00-overview.md` — Vision, naming, crate map, recommended reading order
2. `/Users/will/dev/nunchi/roko/refactoring-prd/01-synapse-architecture.md` — Engram struct, 7-axis Score, 6 Synapse traits (full Rust signatures), universal cognitive loop, three cognitive speeds, dual-process cognition, active inference, cybernetic self-learning loops, composability example
3. `/Users/will/dev/nunchi/roko/refactoring-prd/02-five-layers.md` — L0 Runtime through L4 Orchestration. Trait × layer map. Dependency rules. Adaptive clock. Dual-process tier router. Temperament profiling. Stigmergy. Cross-domain orchestration.
4. `/Users/will/dev/nunchi/roko/refactoring-prd/03-cognitive-subsystems.md` — Neuro (6 knowledge types, 4 tiers, HDC, cross-domain transfer), Daimon (PAD, 6 behavioral states, somatic markers), Dreams (3-phase cycle), Oracles, cybernetic self-learning, VSM mapping (Beer), Good Regulator (Conant & Ashby), Ashby's Law.
5. `/Users/will/dev/nunchi/roko/refactoring-prd/07-implementation-priorities.md` — Current state, Tier 0–6 roadmap, dissolution of roko-golem, dropped items, kept/reframed items, "What Makes This a Series A Story."
6. `/Users/will/dev/nunchi/roko/refactoring-prd/08-translation-guide.md` — Translation rules, incompatibility flags.
7. `/Users/will/dev/nunchi/roko/refactoring-prd/09-innovations.md` — Read §XVII Integration Map, §XVIII Blue Ocean Summary (14 innovations). Brief overview of each innovation.

## Step 3 — Read the SOURCE-INDEX entry for this topic

Use Read to read `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/SOURCE-INDEX.md` and
find the section titled `## 00-architecture.md`. That section lists legacy PRD files,
research/tmp files, implementation-plan files, and reference code files that feed this
topic. **Read every file listed there.** They are the body content — the refactoring-prd
files define the frame.

## Step 4 — Read additional legacy sources

Beyond what's in SOURCE-INDEX, also read these files for the broader vision context:

- `/Users/will/dev/nunchi/roko/bardo-backup/prd/00-vision/00-bardo.md`
- `/Users/will/dev/nunchi/roko/bardo-backup/prd/00-vision/01-thesis.md`
- `/Users/will/dev/nunchi/roko/bardo-backup/prd/00-vision/02-architecture.md`
- `/Users/will/dev/nunchi/roko/bardo-backup/prd/00-vision/03-philosophy.md`
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-refactor/00-overview.md` — "scaffold IS the product" thesis
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-refactor/01-taxonomy.md` — layer taxonomy with citations
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-refactor/12-cognitive-architecture.md` — CoALA, ACT-R, SOAR, dual-process
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-refactor/13-unified-theory.md` — unified theory + scaling laws
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/12-unified-primitives.md` — Signal + 6 traits genesis
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/13-dual-nature-agents.md` — coding + chain composition
- `/Users/will/dev/nunchi/roko/roko/README.md` — current crate descriptions
- `/Users/will/dev/nunchi/roko/roko/CLAUDE.md` — current architecture state

## Step 5 — Also look at the active code

Use Read to examine:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/traits.rs` (if it exists)
- Use Glob to find: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/**/*.rs`
- Read the Engram/Signal definition file to get actual field types

This grounds your writing in the shipping code, not just the spec.

## Step 6 — Plan your sub-docs

Create the output directory:

```bash
mkdir -p /Users/will/dev/nunchi/roko/roko/docs/00-architecture
```

You will write **17 sub-docs** plus one INDEX.md. Each sub-doc addresses one concept and
can be read standalone. The sub-docs are:

| # | Filename | Content |
|---|---|---|
| 00 | `00-vision-and-thesis.md` | "The scaffold IS the product." Meta-Harness (Lee et al. 2026, arXiv:2603.28052). FrugalGPT (Chen et al. 2023, arXiv:2305.05176). DSPy. SWE-bench. CoALA (Sumers et al. 2023, arXiv:2309.02427). What Roko is. Who it's for. The problem it solves. Why context engineering / scaffolding matters more than model selection. |
| 01 | `01-naming-and-glossary.md` | The old→new naming map as a reference document for readers coming from legacy sources. Explain every rename: Bardo→Roko, Mori→Roko Orchestrator, Golem→Agent, Grimoire→Neuro, Styx→Mesh, Clade→Collective/Mesh, GNOS→KORAI/DAEJI, Signal→Engram, "1 noun 6 verbs"→Synapse Architecture. Also define new terms: Engram, Synapse Architecture, C-Factor, KORAI, DAEJI, Spectre, ROSEDUST. |
| 02 | `02-engram-data-type.md` | Full Engram struct (Rust code). Every field. Content addressing via BLAKE3(kind+body+author+tags). Deduplication. Replay. Verification. Cross-system identity. Kind enum with core variants + Custom(String) for domain-extensibility (reverse-DNS prefixes). Body types. Tags as ordered BTreeMap. Created_at_ms. Lineage Vec<ContentHash>. |
| 03 | `03-score-7-axis-appraisal.md` | The 7 score axes: confidence, novelty, utility, reputation (stable) + precision, salience, coherence (extended). Each axis's cognitive function. Effective score formula. Backward compat (new axes default to 0.5). Appraisal theory (Scherer 2001). Why score is separated from ID. |
| 04 | `04-decay-variants.md` | Decay enum: None, HalfLife{half_life_ms}, Ttl{ttl_ms}, Ebbinghaus{strength, scale_ms}. Built-in constants: Decay::THREAT (2h), Decay::OPPORTUNITY (4h), Decay::WISDOM (24h). Ebbinghaus integration with Neuro tier progression. How decay is memory management, not mortality. |
| 05 | `05-provenance-and-attestation.md` | Provenance struct: author, model_fingerprint, prompt_hash, taint level (Trusted/Unverified/Suspicious), timestamp, context. Attestation: Ed25519Signature, PublicKey (DID/TEE), optional ChainAttestation. How attestations enable proving model origin, chain of custody, C2PA-compatible content credentials, regulatory compliance. Taint propagation through lineage DAG. |
| 06 | `06-synapse-traits.md` | Introduction to the 6 Synapse traits. Why they're named after neuroscience. How they compose. Full table with async/sync and primary layer. |
| 07 | `07-substrate-trait.md` | Substrate async trait. Full Rust signature (put/get/query/prune/len/is_empty/name). Implementations: MemorySubstrate (RAM), FileSubstrate (JSONL), ChainSubstrate (Korai on-chain). Storage foundation for everything. |
| 08 | `08-scorer-gate-router-composer-policy.md` | The other 5 traits in detail. Scorer (sync, pure computation). Gate (async, returns Verdict directly not Result, 11+ implementations). Router (sync, Option<Selection>, feedback()). Composer (sync, takes &dyn Scorer, Budget struct). Policy (sync, batch stream input). Full Rust signatures for each. Implementation examples. |
| 09 | `09-universal-cognitive-loop.md` | The 9-step loop: PERCEIVE → EVALUATE → ATTEND → INTEGRATE → ACT → VERIFY → PERSIST → ADAPT → META-COGNIZE. How it maps to CoALA. Loop variants (chain agents add SIMULATE+VALIDATE). How the same loop parameterizes every agent type. |
| 10 | `10-three-cognitive-speeds.md` | Gamma (~5-15s reactive), Theta (~75s reflection), Delta (hours consolidation). The adaptive clock in roko-runtime. How three speeds run concurrently on async tasks. Operating frequencies scheduler. |
| 11 | `11-dual-process-and-active-inference.md` | System 1 / System 2 (Kahneman, CLARION). T0/T1/T2 cascade. Thompson sampling over weighted signals. Active inference (Friston Free Energy Principle). EFE = pragmatic + epistemic value. Zero hyperparameters for explore/exploit. Full EFE formula. How uncertainty drives compute investment. |
| 12 | `12-five-layer-taxonomy.md` | L0 Runtime, L1 Framework, L2 Scaffold, L3 Harness, L4 Orchestration. What each layer does. Primary Synapse traits per layer. Dependencies flow strictly downward. Cross-cuts via trait objects. Trait × layer map. How the loop traverses all layers per tick. |
| 13 | `13-cognitive-cross-cuts.md` | Neuro/Daimon/Dreams as cross-cuts. Plus inference optimization, safety/provenance, observability. Why they're not owned by one layer. How they're injected via Arc<dyn Trait>. |
| 14 | `14-c-factor-collective-intelligence.md` | Two-level metric system. Level 1 ratio (reporting). Level 2 composite C-Score (optimization). Four diagnostic signals. Per-collective, per-agent, global metrics. Woolley et al. Science 330(6004) 2010. |
| 15 | `15-crate-map.md` | The 18+ crates organized by layer. Status (built / scaffold / not started). Test counts. The `roko-golem` dissolution plan. |
| 16 | `16-autocatalytic-and-cybernetics.md` | Autocatalytic improvement (Kauffman). 5 levels of self-improvement. Compound math (0.9^4 = 0.656). Cybernetic self-learning loops. VSM mapping (Beer 1972 Brain of the Firm). Ashby's Law of Requisite Variety. Good Regulator Theorem (Conant & Ashby 1970). Self-model. Theory of mind. Precision-weighted prediction errors. |
| 17 | `17-design-principles-and-frontier-summary.md` | Design principles: dependencies flow down, trait-based API boundaries, cross-cuts injected, domain-specific logic at application layer, everything observable. Brief summary of the 14 blue ocean innovations (point readers to the respective topic docs where each is covered in depth). |

Plus `INDEX.md` — the topic index linking all 18 files.

**Total target**: 18 files. Minimum 200 lines each. Target 500-1500 lines per substantial sub-doc. Total topic size: at least 6,000 lines.

## Step 7 — Write the sub-docs

For each sub-doc:

1. Follow the schema in `context-pack/06-output-structure.md` (## Abstract, main sections, ## Academic foundations, ## Current status and gaps, ## Cross-references).
2. Write for a reader who has never heard of Roko. Define every term on first use.
3. Include Rust code samples from the refactoring-prd and from the active codebase in full — no `// ...` abbreviations.
4. Preserve every citation you encounter in the sources. Format: `Author et al. YEAR (arXiv:ID)` or `(Author et al., Journal Volume(Issue), Year)`.
5. Do not summarize. If a source has 50 lines about a mechanism, your doc has 50+ lines about that mechanism.
6. Do not truncate. If a sub-doc is getting very long, that's fine. If it's getting unwieldy, split into two sub-docs and add both to the INDEX.
7. Apply the naming map and reframe rules throughout.
8. Use the Write tool. Absolute paths starting with `/Users/will/dev/nunchi/roko/roko/docs/00-architecture/`.

## Step 8 — Write the INDEX.md

Follow the INDEX.md schema in `context-pack/06-output-structure.md`. It must:
- Contain a `# Topic 00 — Roko Architecture` heading
- Have an abstract explaining what this topic covers
- Have a Contents table listing all 18 sub-docs with brief descriptions
- Have a Cross-references section
- Have a Key academic foundations section
- Have a Current status section (summarize what's built vs. scaffold vs. missing from the crate map)
- End with a `## Generation Notes` section with metadata

## Step 9 — Self-check

Before finishing, run through `context-pack/04-writing-rules.md` Rule 15 checklist:
- [ ] INDEX.md exists and lists all 17 sub-docs
- [ ] Each sub-doc is at least 200 lines
- [ ] Total topic line count is at least 6,000 lines
- [ ] No instances of "golem" except in rename tables and verbatim quotes
- [ ] No instances of "fleet" in the context of agent groups
- [ ] No instances of "GNOS token"
- [ ] No instances of "Thriving → Terminal" or "terminal requiem" or "Thanatopsis" or "Necrocracy"
- [ ] Every academic citation from the refactoring-prd and legacy sources is preserved
- [ ] The 18-crate map is included
- [ ] All 6 Synapse traits have full Rust signatures
- [ ] The 9-step cognitive loop is documented
- [ ] Three cognitive speeds (Gamma/Theta/Delta) are explained
- [ ] Dual-process T0/T1/T2 is explained
- [ ] 5-layer taxonomy is documented with trait × layer map
- [ ] C-Factor has both level 1 (ratio) and level 2 (composite) formulas
- [ ] Autocatalytic improvement + VSM/Ashby/Good Regulator are explained

## CRITICAL REMINDERS

- **DO NOT SUMMARIZE**. Produce the full substance of every source. If you're about to write "see the original for details" — STOP and include the details.
- **DO NOT TRUNCATE**. The output is long. That's expected. Keep going.
- **PRESERVE ALL CITATIONS**. Every paper, every author, every year.
- **NO DEATH FRAMING**. No mortality. No terminal. No death clocks. No succession. No thanatopsis. (Okay to discuss how they were REMOVED.)
- **USE THE NAMING MAP**. Bardo→Roko, Golem→Agent, Grimoire→Neuro, etc.
- **WRITE FOR ZERO-CONTEXT READERS**. Every term defined on first use.
- **APPLY SYNAPSE ARCHITECTURE LANGUAGE**. Engrams, 6 traits, 5 layers, cross-cuts.
- **DOMAIN-AGNOSTIC FRAMING**. Chain is a domain plugin, not the default.
- **DO NOT ASK FOR CLARIFICATION**. Make decisions per these rules and continue.

You are ready. Start by reading the context pack. Then the refactoring-prd. Then the legacy sources. Then write.
