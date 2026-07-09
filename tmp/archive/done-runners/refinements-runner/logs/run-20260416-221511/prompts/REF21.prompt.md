# Refinements Batch REF21

Run id: run-20260416-221511
Attempt: 3
Model: gpt-5.4
Reasoning: high
Refinement source: /Users/will/dev/nunchi/roko/roko/tmp/refinements/21-from-scratch-redesigns.md
Target docs (candidates): docs/00-architecture/31-implementation-readiness-audit.md docs/00-architecture/23-architectural-analysis-improvements.md

## Previous attempt failure context

Terminology gate failed: retired terms present in changed files.

Recent log tail:
 | roko-fs | 12 | ~1,800 | ~60 | Yes | **Stable** |
 | roko-std | 33 | ~3,500 | ~120 | Yes | **Stable** |
-| Roko runtime | 6 | ~900 | ~12 | Yes | **Stable** |
-| Neuro primitives | 3 | ~500 | 18 | Yes | **Stable** |
+| `roko-runtime` | 6 | ~900 | ~12 | Yes | **Stable** |
+| `roko-primitives` | 3 | ~500 | 18 | Yes | **Stable** |
 | roko-index | 5 | ~700 | 32 | **No** | Built/Unwired |
 | roko-lang-rust | 1 | ~820 | 37 | **No** | Built/Unwired |
 | roko-lang-typescript | 1 | ~918 | 31 | **No** | Built/Unwired |
@@ -641,7 +641,7 @@
 | 04 Verification | 0 (done) | 6 (eval gen, EvoSkills, forensic) | 6 |
 | 05 Learning | 2 (G7) | 4 (ADAS, TrackAndStop) | 6 |
 | 06 Neuro | 2 (G10, G11) | 12 (somatic, cross-domain, library) | 14 |
-| 07 Conductor | 0 (done) | 6 (cognitive signals, L3/L4) | 6 |
+| 07 Conductor | 0 (done) | 6 (cognitive control cues, L3/L4) | 6 |
 | 08 Chain | 0 (deferred) | 24+ (full DeFi stack) | 24+ |
 | 09 Daimon | 2 (G5, G12) | 6 (somatic landscape, contrarian) | 8 |
 | 10 Dreams | 1 (G15) | 10 (REM, HDC counterfactual, hypnagogia) | 11 |

tokens used
101,048
Updated [23-architectural-analysis-improvements.md](/Users/will/dev/nunchi/roko/roko/.roko/worktrees/refinements-run-20260416-221511/docs/00-architecture/23-architectural-analysis-improvements.md), [31-implementation-readiness-audit.md](/Users/will/dev/nunchi/roko/roko/.roko/worktrees/refinements-run-20260416-221511/docs/00-architecture/31-implementation-readiness-audit.md), and [INDEX.md](/Users/will/dev/nunchi/roko/roko/.roko/worktrees/refinements-run-20260416-221511/docs/00-architecture/INDEX.md) for REF21.

`31-implementation-readiness-audit.md` now distinguishes incremental readiness from rewrite readiness, adds the REF21 from-scratch heuristic, the five rewrite-track candidates, governance/risk controls, and explicit rewrite-vs-incremental sequencing. `23-architectural-analysis-improvements.md` now treats the current architecture as a v1 snapshot with a v2 rewrite path, points readers to REF21, and removes the live retired terminology that caused the prior failure. The architecture index summaries now reflect that scope shift.

Verification: `git diff --check` passed. Only `docs/00-architecture/**` files are changed. Required `rewrite` / `from-scratch` vocabulary is present. A final retired-term sweep on the three changed docs returned no matches outside legacy-safe contexts.

New files added: none.  
Retired disclaimers removed: removed live `Signal`/old crate-name wording from the changed docs; no standalone disclaimer blocks were kept.  
Cross-references added: `tmp/refinements/21-from-scratch-redesigns.md` in both target docs, plus glossary references to `docs/00-architecture/01-naming-and-glossary.md`.  
Follow-up: broader terminology drift still exists in other architecture docs and the glossary; if those files are revisited later, they should get the same cleanup.  

Commit-ready message: `refinements(REF21): From-scratch rewrite candidates across implementation-readiness`

=== Finished: 2026-04-16T22:46:19+02:00 ===
=== Duration: 4m 39s ===
=== Exit code: 0 ===
[verify] diff_gate: 3 changed path(s) under docs/
[verify] terminology: scanning 3 file(s)
[verify] terminology violation in docs/00-architecture/23-architectural-analysis-improvements.md: 107:| **CoALA** (Sumers et al. 2023) | 5 memories + 3 action types | Roko's 6 traits subsume CoALA's decomposition |

Use that context to avoid repeating the same failure.

## Shared Context Pack

### 00-REFINEMENTS-RULES

# Refinements Runner — Common Rules (read first)

You are running as an unattended Codex batch from `tmp/refinements-runner`.
Your job: propagate one specific refinement proposal (in `tmp/refinements/`)
into the canonical docs tree under `docs/` so the docs reflect the new
framing.

## Core rules

1. **No prior chat.** This prompt pack must be self-sufficient.
2. **Docs-only.** You may only edit files under `docs/`. Every other path —
   including `crates/**`, `README.md`, `CLAUDE.md`, `tmp/**`, `src/**` — is
   **out of scope**. The verify `scope_gate` fails the batch if any file
   outside `docs/` is touched.
3. **Repository reality only.** Work from files that exist in the worktree.
   Verify current tree with `rg --files` / `rg -n` rather than guessing.
4. **Preserve authority.** The refinement file itself (injected into this
   prompt) is the canonical source. Do not edit it. The docs you touch
   *reflect* it; they do not replace it.
5. **Aggressive edit posture authorised.** You may do full section rewrites
   or full-file rewrites when a doc is deeply misaligned with the
   refinement's framing. Smaller incremental edits are preferred when they
   suffice.
6. **Retired terms must not appear.** Verify-step `terminology_check` will
   fail the batch if retired terms (`Grimoire`, `Styx`, `Clade`, `Mori`,
   `Bardo`, `Golem`, mortal/death/reincarnation framing, "Signal = Engram"
   disclaimers, etc.) appear in lines you introduce — *except* when the
   line explicitly frames them as "retired / deprecated / historical /
   legacy / formerly / renamed / see also / old name". Use that phrasing
   when a retirement table is appropriate.
7. **Cross-link to refinements.** Docs that change should include a
   `see tmp/refinements/NN-slug.md` pointer for readers who want the full
   proposal. Use the exact refinement filename (e.g.
   `tmp/refinements/02-engram-vs-pulse.md`).
8. **Cross-link to glossary.** Docs introducing new terminology should
   point at `docs/00-architecture/01-naming-and-glossary.md` (or wherever
   the glossary has landed).
9. **Subagents authorised.** Every batch includes a "Delegation
   Requirement" section. Use explorers and workers in parallel when the
   target file set is large.
10. **No destructive git.** Never force-push, reset, `rm -rf`. The runner
    handles branch/worktree lifecycle.

## Batch completion bar

A batch is only complete when:

- At least one file under `docs/` is changed (the diff gate fails empty
  batches).
- The batch's required new vocabulary appears in the changed set where
  applicable (per `batch_required_terms` in `lib/common.sh`).
- No retired terms leak into prose outside explicitly retired contexts.
- No file outside `docs/` is touched.
- The new content in each doc is substantive (not "TODO: rewrite later").
- Index docs (`docs/INDEX.md`, `docs/**/INDEX.md`) are updated when new
  chapters are added.

## Failure behaviour

If the batch's scope turns out too broad for the timeout:

- Prioritise the canonical file for this refinement (see batch target
  list) first, then expand outward.
- Land the highest-dependency work before the low-dependency polish.
- Leave a precise note in the final message listing what remains — the
  runner logs it for the next attempt.
- Do not leave half-written sections. If you start rewriting a section,
  finish it.

## Delegation etiquette

When spawning subagents:

- Give each worker a **disjoint** write scope (different files).
- Tell each worker they are **not alone** in the codebase.
- Pass the same context pack (these 6 files) + the same refinement
  source to every subagent.
- Reassemble their work; do not merge half-complete outputs.

## What "aggressive edit posture" means

For a deeply misaligned doc:

- You may replace the entire body with a new version that aligns to the
  refinement.
- Preserve the doc's stable identity: title, filename, anchor IDs used
  elsewhere.
- When you rewrite, cite the refinement source in the opening so readers
  understand the change is intentional and load-bearing.

For a mostly-aligned doc:

- Edit incrementally. Insert new sections where they fit; update stale
  claims; remove stale disclaimers; add cross-references.

Either way: don't produce a half-consistent doc. If the rewrite would
contradict other sections you are not updating in this batch, finish the
other sections too.

## What NOT to do

- Do not touch `tmp/refinements/**` — that's the source. Read-only.
- Do not create new `.md` files outside `docs/`.
- Do not delete large swaths of content without replacing it — the doc
  stays load-bearing after your edit.
- Do not reference files that do not yet exist without an explicit "(to
  be added in batch REFxx)" marker.
- Do not add content that contradicts other refinements that have
  already landed. If you find a contradiction, document it in the final
  message and proceed with the simplest consistent edit.

## Context paths

Always read these files before editing:

1. `tmp/refinements-runner/context-pack/00-REFINEMENTS-RULES.md` (this)
2. `tmp/refinements-runner/context-pack/01-TWO-FABRIC-PRIMER.md`
3. `tmp/refinements-runner/context-pack/02-TERMINOLOGY-TABLE.md`
4. `tmp/refinements-runner/context-pack/03-DOCS-TREE-MAP.md`
5. `tmp/refinements-runner/context-pack/04-SYNERGY-SUMMARY.md`
6. `tmp/refinements-runner/context-pack/05-REFINEMENTS-INDEX.md`

And the canonical refinement file for this batch (injected below under
"Canonical refinement source").

### 01-TWO-FABRIC-PRIMER

# The Two-Fabric Framing — Primer

Every batch needs this vocabulary. It is the shared mental model that
the 35 refinements propagate into `docs/`.

## The one-liner

> Roko's kernel is two mediums (**Engram** — durable, content-addressed,
> decayed; **Pulse** — ephemeral, topic-addressed, sequenced) moving
> through two fabrics (**Substrate** — storage; **Bus** — transport),
> acted on by six operators (**Scorer**, **Gate**, **Router**,
> **Composer**, **Policy**, plus the fabric traits). Layers enforce a
> downward-only dependency rule. Three cognitive speeds (Gamma, Theta,
> Delta) run the loop; three cross-cuts (Neuro, Daimon, Dreams) inject
> across layers.

## Canonical term definitions

**Engram** — Durable medium. Content-addressed by BLAKE3 over
`(kind, body, author, tags)`. Has `lineage: Vec<ContentHash>`,
`decay` / `balance`, `score` (7 axes), `provenance`, optional
`attestation`, and (post-refinement) an HDC `fingerprint`. Lives in a
Substrate.

**Pulse** — Ephemeral medium (new). Typed, topic-addressed,
sequence-numbered, ring-buffered message on a Bus. Not content-
addressed; not persisted by default. May *graduate* to an Engram when
its lineage matters.

**Substrate** — Storage fabric (kernel trait). Backends: Memory, File,
HDC, Chain. Persists Engrams. Retrieval by filter or (new)
HDC similarity.

**Bus** — Transport fabric (promoted to kernel trait). Backends:
BroadcastBus (in-process), MultiBus, NatsBus / KafkaBus
(multi-process, Phase 2+), ChainBus (on-chain events, Phase 2+).
Publishes and delivers Pulses by Topic.

**Topic** — Routing handle for Pulses. Dot-separated lowercase
strings (`gate.verdict.emitted`, `agent.msg.chunk`, `prediction.error`).

**Datum** — `enum Datum<'a> { Engram(&'a Engram), Pulse(&'a Pulse) }`
used by polymorphic operators (Scorer, Composer, Router).

**Six operators** — `Scorer`, `Gate`, `Router`, `Composer`, `Policy`,
plus the fabric traits `Substrate` and `Bus` (which act as storage
and transport operators themselves).

## The loop (revised)

The universal cognitive loop becomes 7 steps, with co-equal PERSIST
and BROADCAST:

```
1. SENSE      — Substrate.query | Bus.subscribe | external I/O
2. ASSESS     — Scorer + Router pick what to do
3. COMPOSE    — Composer assembles a prompt Engram under a budget
4. ACT        — execute (LLM | tool | chain); produces Pulses + final Engram
5. VERIFY     — Gate pipeline + optional stream-gates; emits Verdict Engram
6. PERSIST    — Substrate.put (Engrams)
   BROADCAST  — Bus.publish (Pulses, in parallel)
7. REACT      — Policies react, emit new Pulses + Engrams
```

Cross-cuts are NOT loop steps. Neuro, Daimon, and Dreams inject into
operators at specific steps, not into the loop sequence.

## The five layers

L0 Runtime (Substrate, Bus, process supervisor, cancellation, clock)
L1 Framework (tools, routing, safety)
L2 Scaffold (context, composition)
L3 Harness (gates, monitoring)
L4 Orchestration (plan DAG, scheduling)

Strictly downward dependencies. The Bus (promoted) joins Substrate at
L0.

## Three speeds

Gamma (~5-15 s) — turn-level. Token streams, quick gates, live
context.

Theta (~75 s) — plan-level. Full gate pipeline, episode consolidation,
routing learning.

Delta (hours) — background. Dreams consolidation, tier progression,
meta-template optimization.

Same seven-step loop at all three speeds; only scope, budget, and
persistence cadence change.

## Three cross-cuts

**Neuro** — Durable knowledge store, distillation, tier progression.
Formerly Grimoire. Inject into Substrate reads (step 1) and Composer
(step 3).

**Daimon** — PAD-vector affect. Formerly loop "step 9 META-COGNIZE."
Inject into Scorer bias (step 2) and Action gate (step 4).

**Dreams** — Offline consolidation loop at Delta speed. Inject
consolidated knowledge back into Substrate for the next cycle.

## Naming — quick reference

| Term | Use | Avoid |
|---|---|---|
| Engram | the durable record | Signal (retired in code 877:5) |
| Pulse | the ephemeral message | Event, Envelope, Message, Signal |
| Bus | the transport trait | EventBus<E> as a trait name |
| Substrate | the storage trait | (unchanged) |
| Topic | routing handle | Channel, Subject |
| Datum | polymorphic reference | `&Any`, variant enums |
| Neuro | cross-cut | Grimoire |
| Mesh | agent-network layer | Styx |
| Fleet | agent roster | Clade |
| Roko | the project | Bardo, Golem, Mori |

Never use "mortal", "death", "reincarnation" framing. Retired.

### 02-TERMINOLOGY-TABLE

# Terminology Table (retired → current)

This table is the single source of truth for what terminology must
appear / not appear in updated docs. The verify step (`terminology_check`
in `lib/verify.sh`) greps for the "retired" column and fails when any
match lands outside a line that also signals "retired", "deprecated",
"historical", "formerly", "legacy", "old name", "renamed", or
"see also".

## Retired → use instead

| Retired term | Use instead | Notes |
|---|---|---|
| `Signal` (as the durable record) | `Engram` | Rename landed in code (877 Engram vs 5 Signal). Docs should say Engram. |
| `Signal is the same as Engram` | (delete disclaimer) | The equivalence disclaimer is stale. Remove. |
| `EventBus<E>` (as trait name) | `Bus` + `Pulse` | The generic struct stays as `BroadcastBus` implementation detail. |
| `Envelope<E>` (as user-facing type) | `Pulse` | Envelope name retained only as internal impl detail. |
| `Event` (as type name) | `Pulse` | Collides with tokio/winit. Use Pulse. |
| `Message` (as Roko type name) | `Pulse` (wire) / `ChatMessage` (LLM) | Overloaded term. |
| `Grimoire` | `Neuro` | Cross-cut; durable knowledge store. |
| `Styx` | `Mesh` + `Korai` | Split into mesh (network) + Korai (chain). |
| `Clade` | `Fleet` | Agent roster. |
| `Bardo` | `Roko` | Project name. |
| `Golem` | `Agent` | Agent session/process. |
| `Mori` | `Roko` | Project name. |
| `mortal` / `death as <x>` / `reincarnation` | (remove framing) | Retired thematic framing. |

## Allowed contexts for retired terms

Lines that explicitly frame the retirement are allowed:

- "formerly Grimoire"
- "renamed from Signal"
- "historical: Styx"
- "see also: the retired term `Event`"
- "deprecated: `EventBus<E>` — use the `Bus` trait"
- "old name: Bardo (pre-2026)"
- A retirement table column labelled "retired" or "legacy"

The verify step allows these by matching a loose set of context markers
in the same line (case-insensitive): `retired`, `deprecated`,
`historical`, `formerly`, `legacy`, `old name`, `see also`, `renamed`.

## New term quick reference (where they should appear)

| New term | Must appear after batch | Home refinement |
|---|---|---|
| `Pulse` | REF02, REF07 | 02-engram-vs-pulse.md |
| `Bus` (as kernel trait) | REF03 | 03-bus-as-first-class.md |
| `Topic` | REF03, REF07 | 03-bus-as-first-class.md |
| `TopicFilter` | REF03, REF07 | 03-bus-as-first-class.md |
| `Datum` | REF04 | 04-operators-generalized.md |
| `PulseSource` | REF02, REF07 | 02-engram-vs-pulse.md |
| `two mediums` | REF01, REF02 | 01-critique-one-noun.md |
| `two fabrics` | REF03 | 03-bus-as-first-class.md |
| `seven-step loop` | REF05 | 05-loop-retold.md |
| `demurrage` | REF12 | 12-knowledge-demurrage.md |
| `heuristic` (as first-class kind) | REF14 | 14-worldview-validation.md |
| `c-factor` | REF13 | 13-collective-intelligence-c-factor.md |
| `HDC fingerprint` | REF11 | 11-hyperdimensional-substrate.md |
| `replication ledger` | REF16 | 16-research-to-runtime.md |
| `StateHub projection` | REF26 | 26-statehub-rearchitecture.md |
| `TypedContext` | REF25 | 25-domain-specific-agents.md |
| `Custody` (chain-of-custody) | REF32 | 32-safety-sandbox-provenance.md |
| `synergy matrix` | REF31 | 31-synergy-integration-map.md |

## Cross-references between docs

Every refinement propagation should include a trailing "See also" or
inline cross-reference to the refinement file:

- `see [02-engram-vs-pulse](../../tmp/refinements/02-engram-vs-pulse.md)`
- or `see tmp/refinements/02-engram-vs-pulse.md §3` for section-specific
  references.

Use the project's existing link conventions where a doc subfolder has
them. When in doubt, absolute repo-relative paths are always correct.

### 03-DOCS-TREE-MAP

# docs/ Tree Map

The `docs/` directory has 23 subdirectories and ~405 Markdown files.
This map tells you where each subsystem lives so you can find the right
file to edit for a given refinement.

## Top-level files

- `docs/INDEX.md` — master index. Linked from the repo root.
- `docs/VISION-RUN-ANYWHERE.md` — deployment vision.

## Sub-chapters (organised by architecture topic number)

| Folder | Topic | Notes |
|---|---|---|
| `docs/00-architecture/` | **Core architecture** (34 files including INDEX) | The canonical architecture chapter. Most refinements rewrite files here. |
| `docs/01-orchestration/` | Plan runner, DAG, merge queue | Touched by REF05, REF06 |
| `docs/02-agents/` | Agent runtime, backends, roles | Touched by REF22, REF23, REF25 |
| `docs/03-composition/` | Composer, templates, enrichment | Touched by REF04, REF14 |
| `docs/04-verification/` | Gate pipeline, rungs | Touched by REF04, REF05, REF32 |
| `docs/05-learning/` | Episodes, playbooks, bandits, experiments | Touched by REF10, REF12, REF14, REF16 (heavy) |
| `docs/06-neuro/` | Durable knowledge store, distillation | Touched by REF11, REF12, REF14 |
| `docs/07-conductor/` | Watchers, circuit breakers, diagnosis | Touched by REF03 (layer violation dissolves), REF33 |
| `docs/08-chain/` | Chain integration (Phase 2+) | Touched by REF09 (ChainBus + ChainSubstrate split) |
| `docs/09-daimon/` | PAD affect, behavior gates | Touched by REF04, REF10 |
| `docs/10-dreams/` | Offline consolidation | Touched by REF09 |
| `docs/11-safety/` | Safety layer, tool auth, taint | Touched by REF32 (heavy) |
| `docs/12-interfaces/` | CLI, TUI, web, HTTP API | Touched by REF22, REF23, REF24, REF25, REF26, REF27, REF28, REF29, REF30 (heaviest) |
| `docs/13-coordination/` | Stigmergy, mesh, c-factor | Touched by REF09, REF13 |
| `docs/14-identity-economy/` | Attention economy, tokens | Touched by REF12 |
| `docs/15-code-intelligence/` | Code-intelligence MCP | Touched by REF17 |
| `docs/16-heartbeat/` | Three cognitive speeds, clock | Touched by REF05, REF10 |
| `docs/17-lifecycle/` | Agent lifecycle, sessions | Touched by REF23 |
| `docs/18-tools/` | Tools, MCPs, plugins | Touched by REF17, REF25 |
| `docs/19-deployment/` | Deployment shapes, ops | Touched by REF24, REF27, REF33 |
| `docs/20-technical-analysis/` | Moat, scaling, innovations | Touched by REF15, REF18, REF19 |
| `docs/21-references/` | Papers, citations | Touched by REF16 |

## docs/00-architecture/ (the big one)

This folder has the canonical numbered architecture chapter. When a
refinement affects "the architecture story," these are usually the files
to update:

- `00-vision-and-thesis.md`
- `01-naming-and-glossary.md` — every naming-related refinement touches this
- `02-engram-data-type.md` — REF02
- `03-score-7-axis-appraisal.md` — mostly stable
- `04-decay-variants.md` — REF12 (supersedes decay with demurrage)
- `05-provenance-and-attestation.md` — REF32
- `06-synapse-traits.md` — REF01, REF04
- `07-substrate-trait.md` — REF03 (split into 07a + 07b)
- `08-scorer-gate-router-composer-policy.md` — REF04
- `09-universal-cognitive-loop.md` — REF05 (rewrite to 7 steps)
- `10-three-cognitive-speeds.md` — mostly stable
- `11-dual-process-and-active-inference.md` — REF10
- `12-five-layer-taxonomy.md` — REF03, REF20
- `13-cognitive-cross-cuts.md` — REF05 (cross-cuts inject into ops)
- `14-c-factor-collective-intelligence.md` — REF13
- `15-crate-map.md` — REF20
- `16-autocatalytic-and-cybernetics.md` — REF10, REF15
- `17-design-principles-and-frontier-summary.md` — REF18, REF19
- `18-decay-tier-matrix.md` — REF12
- `19-compositional-kinds.md` — REF02, REF04
- `20-configuration-schema.md` — REF12 (demurrage rates), REF14
- `21-performance-numerical-stability.md` — REF33
- `22-error-handling-recovery.md` — mostly stable
- `23-architectural-analysis-improvements.md` — REF01, REF04, REF20, REF21
- `24-cross-section-integration-map.md` — REF03, REF09, REF26, REF31
- `25-attention-as-currency.md` — REF12
- `26-cognitive-immune-system.md` — REF32
- `27-temporal-knowledge-topology.md` — REF11
- `28-emergent-goal-structures.md` — mostly stable
- `29-cognitive-energy-model.md` — REF15
- `30-cross-pollination-innovations.md` — REF15, REF18, REF19
- `31-implementation-readiness-audit.md` — REF06, REF21, REF35
- `32-comprehensive-test-strategy.md` — REF33
- `INDEX.md` — most refinements

## INDEX conventions

Every subdir has an `INDEX.md`. When you add a new chapter or change a
chapter's scope, update the relevant INDEX.md. The top-level
`docs/INDEX.md` aggregates; update it when the change is cross-chapter.

## When you can't find the right home for new content

If a refinement introduces a new concept (e.g. `heuristic` as a
first-class memory kind in REF14), either:

1. Add it to an existing file where it naturally fits (preferred for
   small additions), OR
2. Add a new file in the right subdirectory with a natural number that
   doesn't collide. Look at the existing numbering to pick the next
   stable slot.

Do NOT add files in `docs/00-architecture/` with numbers already in use.

## What's NOT in docs/ (and why to leave it alone)

- `crates/**` — source code. Never touch.
- `README.md` (repo root) — out of scope for this runner.
- `CLAUDE.md` — out of scope.
- `tmp/refinements/**` — read-only source.
- `tmp/ux-followup/**`, `tmp/MASTER-PLAN.md` — out of scope.

If you want to cross-link from a doc you're editing to e.g. a crate or
CLAUDE.md, use a relative repo path like `../../crates/roko-core/` —
that's a reference, not an edit.

### 04-SYNERGY-SUMMARY

# Synergy Summary

Condensed from `tmp/refinements/31-synergy-integration-map.md`. Use this
when a batch needs to know how the primitive it's propagating interacts
with the others.

## Ten load-bearing primitives

1. **Engram** (durable medium) — home: REF02
2. **Pulse** (ephemeral medium) — home: REF02
3. **Bus** (transport fabric) — home: REF03
4. **Substrate** (storage fabric) — home: REF03
5. **HDC fingerprint** — home: REF11
6. **Demurrage** (attention economy) — home: REF12
7. **Heuristics + falsifiers** — home: REF14
8. **c-factor** (collective intelligence) — home: REF13
9. **Replication ledger** — home: REF16
10. **Plugin SPI + domain profiles** — homes: REF17, REF25

## Named synergies (ten concrete interactions)

When propagating any one of these primitives into docs, cross-link to
the ones it composes with:

- **Demurrage × HDC → self-trimming semantic memory.** Substrate +
  HDC + Demurrage combined produce memory that self-prunes toward
  unique-and-used. Cite in REF11, REF12.
- **Heuristic × Pulse × Bus → continuous calibration.** Falsifier
  Pulses close the loop. Cite in REF14.
- **c-factor × Bus × HDC → diversity-aware routing.** Cite in REF13.
- **Replication ledger × Heuristics × Paper → living research.** Cite
  in REF14, REF16.
- **Plugin SPI × Substrate × Bus → ecosystem growth.** Cite in REF17.
- **c-factor × Heuristics → peer-model learning.** Cite in REF13, REF14.
- **Dreams × Substrate × Pulse → retroactive insight.** Cite in REF09.
- **Demurrage × Heuristic × Calibration → graceful relearning.** Cite
  in REF12, REF14.
- **HDC × Consensus × Bus → substantive agreement.** Cite in REF11,
  REF13.
- **TypedContext × Domain × Gate → auditable domain safety.** Cite in
  REF25, REF32.

## The moat claim (short form)

Feature-level competition copies any single primitive in weeks. The
*composition* of all 10 primitives into one coherent system is a
multi-year project. Refinement docs in `docs/20-technical-analysis/`
should include this framing.

## The loop across the primitives

The seven-step loop (SENSE, ASSESS, COMPOSE, ACT, VERIFY, PERSIST,
BROADCAST, REACT) uses every primitive:

- SENSE — Substrate (query) + Bus (subscribe) + external I/O.
- ASSESS — Scorer (weights via HDC/demurrage/heuristic calibration).
- COMPOSE — Composer (picks from HDC-similar Engrams, injects heuristics,
  TypedContext shapes prompt).
- ACT — produces Pulses (agent stream, tool calls).
- VERIFY — Gate pipeline; gate-verdict Pulses feed heuristic calibration.
- PERSIST — Substrate put, demurrage balance assigned; HDC fingerprint
  computed; Custody record written for auditable actions.
- BROADCAST — Bus publish, StateHub projections fold deltas.
- REACT — Policy decides; c-factor observers log; replication ledger
  watchdog checks claim falsifiers.

Every step references multiple primitives. Cross-reference accordingly
when writing prose for any refinement.

### 05-REFINEMENTS-INDEX

# Refinements Index — 35 Proposals

One-sentence summary of every refinement in `tmp/refinements/`. Use
this when a batch needs to know what a neighboring refinement is about
without reading its full source.

## Foundation (01–09)

- `01-critique-one-noun.md` — Diagnosis: "one noun, six verbs" framing
  conflates two data shapes, hides the event bus, stretches trait
  signatures.
- `02-engram-vs-pulse.md` — Introduce Pulse (ephemeral) as Engram's
  sibling; define graduation law.
- `03-bus-as-first-class.md` — Promote Bus to a kernel trait at L0
  alongside Substrate.
- `04-operators-generalized.md` — Generalize the six operators over a
  `Datum` enum that is either Engram or Pulse.
- `05-loop-retold.md` — Universal loop collapses from 9 to 7 steps;
  PERSIST and BROADCAST become co-equal; cross-cuts aren't steps.
- `06-refactoring-plan.md` — Three-phase refactor (docs → kernel →
  subsystem migration); 6–7 weeks total.
- `07-naming.md` — Name the new ephemeral type `Pulse` (not `Event`,
  not `Signal`). Bus, Topic, TopicFilter, Datum.
- `08-code-sketches.md` — Concrete Rust: Pulse type, Bus trait, Datum
  enum, graduation, conductor port.
- `09-phase-2-implications.md` — Chain, Dreams, Mesh, Coordination
  (stigmergy), Heartbeat, HTTP control plane all become Bus consumers.

## Learning, intelligence, moat (10–21)

- `10-self-learning-cybernetic-loops.md` — Every operator becomes a
  predictor; active inference literal via predict/outcome Pulses.
- `11-hyperdimensional-substrate.md` — 10,240-bit HDC fingerprint on
  every Engram; similarity/consensus/analogy as O(1) vector ops.
- `12-knowledge-demurrage.md` — Economic memory: balance, holding
  cost, reinforcement-by-kind; self-trimming playbooks.
- `13-collective-intelligence-c-factor.md` — Woolley's c-factor
  measured continuously from Bus statistics; Policy optimizes it.
- `14-worldview-validation.md` — Heuristics with explicit falsifiers;
  worldviews as co-citation clusters; lived-experience calibration.
- `15-exponential-scaling.md` — Seven compounding loops; "every week
  your Roko gets better on your codebase."
- `16-research-to-runtime.md` — Papers as Engrams, Claims as testable
  hypotheses, Replication Ledger — living research.
- `17-plugin-extension-architecture.md` — Five-tier SPI (prompts,
  profiles, manifests, native, WASM) with matched sandboxes.
- `18-competitive-moat.md` — Five structural components: coherence,
  heuristic commons, ecosystem, replication ledger, Rust correctness.
- `19-net-new-innovations.md` — Flat catalog of primitives with no
  known prior art.
- `20-modularity-composability.md` — Proposed dep graph; three new
  kernel crates (roko-bus, roko-hdc, roko-spi).
- `21-from-scratch-redesigns.md` — Five rewrite candidates with cost/
  unlock analysis and 2-month sequencing.

## UX (22–30)

- `22-developer-ux-rust.md` — Four-layer Rust SDK (one-liner / builder /
  trait / runtime) for Rust devs building agents.
- `23-user-ux-running-agents.md` — One verb-set, four surfaces (CLI /
  TUI / Chat / Web); interactive first-run.
- `24-deployment-ux.md` — Five deployment shapes (laptop /
  single-server / container / clustered / edge).
- `25-domain-specific-agents.md` — Six domain profiles (coding,
  research, blockchain, data, ops, writing) + TypedContext + Custody.
- `26-statehub-rearchitecture.md` — Promote StateHub from TUI helper
  to kernel projection layer; typed, filterable, multi-consumer.
- `27-realtime-event-surface.md` — WebSocket / SSE / gRPC with a
  single wire protocol; first-party clients.
- `28-cli-parity-familiar-workflows.md` — Claude Code / Aider muscle
  memory; slash commands, diff-first, per-hunk control, transcripts.
- `29-web-ui-architecture.md` — Five-page first-party web UI on
  SvelteKit + StateHub.
- `30-rich-ux-primitives.md` — Ten UX primitives: reasoning streams,
  tool banners, heuristic footnotes, replay scrubber, uncertainty bars.

## Integrators (31–35)

- `31-synergy-integration-map.md` — 10×10 matrix showing how the
  primitives reinforce each other. The moat is the interaction density.
- `32-safety-sandbox-provenance.md` — Safety spine: role auth, tier
  sandboxes, custody, taint, attestation, multi-tenancy.
- `33-observability-telemetry.md` — Logs / metrics / traces / events /
  replay / cost; Roko-specific metrics like `roko.c_factor`.
- `34-glossary.md` — A–Z vocabulary of every new and retired term.
- `35-consolidated-roadmap.md` — Dependency graph and Q1–Q4 sequencing
  across all refinements.

## Reading shortcut

If a batch's refinement depends on earlier ones, the batch's `deps` in
`lib/common.sh` is non-empty. The runner's DAG ensures they've landed
before. When your prompt talks about Pulse (REF02) as assumed
vocabulary, REF02 is already done in the current worktree.

## Delegation Requirement

You are authorized to use subagents. Prefer multiple parallel agents when
the target docs set is large.

Required delegation behavior:

- Form a plan first — for each candidate `docs/` file listed in the batch,
  decide (a) does it need changes, (b) how big, (c) is it self-contained.
- For large independent files, spawn a worker per file with a disjoint
  write scope.
- Every subagent gets the same context pack and the same refinement source.
- Do not wait idly for subagents if you can progress locally.
- If subagents are unavailable in this environment, continue locally.

Suggested parallel split for batch `REF21`:

- worker: update `docs/00-architecture/31-implementation-readiness-audit.md`
  with the from-scratch candidates and sequencing.
- worker: update `docs/00-architecture/23-architectural-analysis-improvements.md`
  with a pointer to the rewrite list.

## Canonical refinement source

This is the verbatim refinement proposal that this batch must propagate
into `docs/`. Treat it as the authoritative source. Do not edit this
file; only edit docs under `docs/`.

--- BEGIN 21-from-scratch-redesigns.md ---

# From-Scratch Redesigns

> **TL;DR**: Some of the refinements in this folder can be
> retrofitted. Others benefit from *removing* existing code and
> designing the replacement clean. This doc lists five candidate
> from-scratch rewrites, for each: what to remove, what to replace
> it with, what it unlocks, what it costs, and whether it's worth
> it. This is the "what would you do with an afternoon to break
> things" list — used sparingly, but occasionally a clean rewrite
> saves six months of incremental patching.

> **For first-time readers**: The refactoring plan in 06 is
> incremental — no subsystem breaks until Phase C, and even then via
> compiler-assisted signature updates. This doc is the contrasting
> "what would we rewrite clean?" list. Five candidates, each scored
> against a heuristic for when from-scratch beats incremental.
> Read 06 first; this is the alternative sequencing where worth it.

## 1. When a from-scratch is justified

Heuristic: rewrite from scratch when at least three of these are
true:

1. The current design embeds an *assumption* you want to invert
   (e.g., "one noun" when you want "two mediums").
2. The code is <10K lines — too small to warrant preservation
   costs.
3. The interface surface is small or can be held stable.
4. The new design unlocks a capability that incremental refactoring
   cannot reach cleanly.
5. The old design has few direct users and no plugin contracts.

Otherwise: incremental refactor wins. Rewriting is not a virtue;
it is a tool.

## 2. Candidates

### 2.1 `roko-core` kernel

**Current**: 1 noun (Signal/Engram), 6 traits, stable shape.
**Replace with**: 2 mediums (Engram + Pulse), 7 operators
(Substrate, Bus, Scorer, Gate, Router, Composer, Policy), per `02`,
`03`, `04`.
**Unlocks**: Bus as first-class, unified operator trait over either
medium, coherent kernel.
**Costs**: 2–3 weeks of careful work. ~15 consumer crates need
import updates. Public API break requires semver-major bump.
**Worth it?**: Yes — the current framing actively misrepresents
the system. Every week the current kernel persists, more code
takes an implicit dependency on "Engram only" that later has to
be undone.

### 2.2 `roko-learn` reorganization

**Current**: episodes, playbooks, bandits, experiments, efficiency,
cascade router, all in one crate with mixed concerns.
**Replace with**: five focused crates:
- `roko-episode` (record, store, query)
- `roko-playbook` (distillation, retrieval)
- `roko-bandit` (arm management, Thompson/UCB)
- `roko-experiment` (A/B, experiment store)
- `roko-heuristic` (new — see `14`)

Plus *learning-as-subscription*: each crate subscribes to Bus
topics and reacts. No crate is called explicitly; they self-wire.

**Unlocks**: cleaner dep graph (see `20`), ability to replace any
one learning strategy without touching others, easier plugin
contributions.
**Costs**: significant churn — ~3K lines rearranged across
five crates. Two weeks of work. No public API break if the CLI
retains its current shape.
**Worth it?**: Yes, but after the kernel rewrite. Doing it before
is wasted work if kernel types change.

### 2.3 Substrate trait rewrite

**Current**: `Substrate::get/put/delete/list`.
**Replace with**: `Substrate::put/get/query/scan/freeze/thaw`, with:
- `query(predicate)` for HDC-similarity and filter-based queries
- `scan(range)` for lineage walks
- `freeze(hash)` + `thaw(hash)` for demurrage-driven cold tier
- subscribe-style notifications via Bus

**Unlocks**: uniform content-addressed + similarity query, cold
tier (see `12`), replicable substrate semantics across storage
backends.
**Costs**: 1 week. Affects every call-site. Public API break for
the Substrate trait.
**Worth it?**: Yes — the current surface is too minimal for the
memory primitives we want. Forcing consumers to build their own
query layers produces divergent implementations.

### 2.4 Gate pipeline

**Current**: 11 gates in a 7-rung pipeline with adaptive
thresholds. Implementation is a roll-your-own state machine.
**Replace with**: gates as pure functions + composition combinators
(sequence, parallel, any, all, retry, budget). Thresholds are a
property of the *composition*, not the gate.

**Unlocks**: third-party gates as simple Rust functions or WASM
modules, testability (gates are pure), visualizability (the
composition is inspectable data).
**Costs**: 1–2 weeks. Existing gates need lightly adapting. Gate
results and reason strings need normalization.
**Worth it?**: Maybe. Gates are already working; this is cleaner
but not unlocking a specific user-facing capability. Schedule it
for a slow month.

### 2.5 `roko-compose` engine

**Current**: 6-layer prompt builder with roles loaded from
template files, enrichment hooks.
**Replace with**: a *query-driven* compose pipeline — the system
prompt is not built from a fixed template, it's assembled from
whichever Engrams match a query (role, situation fingerprint,
relevant heuristics, recent episodes). Template is a query, not a
string.

**Unlocks**: dramatically more dynamic system prompts, plug-in
templates become data queries, per-situation prompt specialization
via HDC retrieval.
**Costs**: 2 weeks. Old template files become fallback defaults.
Consumers that depend on deterministic prompts need a stable-mode
flag.
**Worth it?**: Yes, long-term — this is where the HDC substrate
pays off dramatically. Short-term the existing engine is fine.
Schedule after the substrate rewrite (2.3).

## 3. The "no rewrite" list

Things that are tempting to rewrite but shouldn't be:

- **roko-fs**: stable, JSONL-on-disk, well-scoped. A Postgres
  backend is a new crate, not a rewrite.
- **roko-cli**: churn all we like; no rewrite. The subcommand
  structure is earning its keep.
- **roko-runtime**: except for the bus extraction in 2.1, leave
  alone. Supervisor + cancellation semantics are hard-won.
- **roko-agent dispatcher**: the backend fan-out is working and
  extensible; no need.
- **roko-orchestrator**: DAG + parallelism + merge queue is
  complex and right; extending, not rewriting.

## 4. The meta-rewrite: the docs

Docs aren't code but they benefit from the same logic. Two rewrites
already queued:

### 4.1 `docs/00-architecture/` chapter 1

Rewrite the foundational story to reflect "two mediums, six
operators." This is the payoff of this whole folder. Three to five
refreshed pages replacing the current one.

### 4.2 CLAUDE.md and README

Update the "1 noun + 6 verbs" line. It's on the literal front page
of the project. Every day it stays there, the wrong mental model
propagates. Rewrite after the kernel rewrite lands.

## 5. Sequencing

Optimal order:

```
1. roko-bus extraction              (week 1)  [from 20]
2. Kernel rewrite (2.1)             (weeks 2–4)
3. Substrate rewrite (2.3)          (weeks 4–5)
4. Docs rewrite (4.1, 4.2)          (week 5)
5. roko-learn reorganization (2.2)  (weeks 6–7)
6. Compose rewrite (2.5)            (weeks 8–9)
7. Gate rewrite (2.4) — if desired  (week 10)
```

Two months of focused rewrite work. At the end, Roko has a
substantially cleaner kernel and the rest of the refinements in
this folder can land on top of it cleanly.

## 6. Risk management

Rewriting anything in a production codebase is a risk. Mitigations:

### 6.1 Feature-flag the new kernel

Build both old and new types for one release cycle. `--kernel=v2`
flag exercises the new path. Gives time to shake out bugs without
a hard cutover.

### 6.2 Compatibility shim for Engram format

Old Engrams on disk should load into the new kernel. A one-shot
migration reader, not a dual-world permanent compat layer.

### 6.3 Extensive test parity

Before rewriting, record current test outputs. After rewriting,
outputs must match or the difference must be explicitly justified.
Test-driven refactoring at the crate level.

### 6.4 Sequenced landing

Never merge two rewrites in the same week. Land one, bake for days,
then the next. Rushing this is how multi-month regressions happen.

## 7. What we gain by committing

A version of Roko with the refinements in this folder — two
fabrics, HDC-everywhere, demurrage, heuristics, c-factor,
replication ledger, plugin tiers — is substantially different from
the current Roko. It is worth thinking about this as a *second
version of Roko*, not a refactoring. The kernel rewrite is the
moment that distinction is acknowledged.

That framing also helps planning: the current codebase is a
successful v1. The refinements are the design of v2. We can
deliberately choose which v1 behaviors carry forward and which
are replaced.

## 8. What we risk by not committing

If we don't do the rewrites, each refinement lands as a patch on
top of assumptions it contradicts. Demurrage clashes with the
current decay field. HDC clashes with the current content-address
query. Heuristics clash with the current playbook store. Each
patch works; together they form a Frankenstein. A Frankenstein
system is shippable but loses the architectural-coherence moat
described in `18`.

The worst outcome is a Roko that *has* these features but *is* a
patch quilt. We'd lose the defensibility claim while still paying
the complexity cost. Better to commit to the kernel rewrite or
commit to not doing the refinements, rather than straddling.

## 9. Recommendation

Commit. Do the rewrites in the order above. Two months of focused
work, then the refinements in docs 2–19 land on the new foundation
cleanly and accrue the moat. The alternative — patching
indefinitely — produces a system that's harder to explain, harder
to extend, harder to defend.

The sunk cost of existing code should not overweight the
opportunity cost of the superior design. Especially when the
existing code is 18 crates and 177K lines — a scale where
rewriting the kernel *is* feasible, unlike at 10x scale where it
would not be.

## 10. The "not a rewrite" in each candidate

Even the most aggressive of the five candidates (2.1, kernel rewrite)
keeps a lot intact. For the record, the things that *don't* change in
each:

- **Kernel rewrite (2.1)** — Engram struct and its persistence format.
  BLAKE3 content hashing. The 7-axis Score. The Decay enum. Provenance
  and attestation shapes. The existing 131 trait impls mostly compile
  unchanged; signatures widen, bodies stay.
- **Learn reorganization (2.2)** — bandit algorithms, cascade-router
  logic, experiment store math. The crate-reshaping is mechanical.
- **Substrate rewrite (2.3)** — BLAKE3 addressing, JSONL format on
  disk, lineage field, scoring pipeline. The `query_similar` adds; the
  existing `query` stays.
- **Gate rewrite (2.4)** — existing gate implementations (compile,
  test, clippy, diff). The state-machine/composition change is about
  orchestration, not the gates themselves.
- **Compose rewrite (2.5)** — layer types, template content,
  enrichment hooks. The engine changes; the templates as data stay.

The rewrites are structural, not content-level. That's what makes
them tractable.

## 11. Risk ranking of the five

Explicitly scoring risk so sequencing reflects it:

| # | Candidate | Risk of breakage | Mitigating factor |
|---|---|---|---|
| 2.1 | Kernel rewrite | High | Feature-flag path per 6.1; 2-week bake |
| 2.2 | Learn reorganization | Low | Mostly crate-reshaping |
| 2.3 | Substrate rewrite | Medium | JSONL format preserved |
| 2.4 | Gate rewrite | Medium-high | Changes result shapes |
| 2.5 | Compose rewrite | High | Prompt determinism suffers |

The §5 sequencing respects this: kernel (2.1) rewrite first while
the team has the most energy, gate (2.4) scheduled last because it's
in the "maybe" column anyway. 2.3 lands right after the kernel
because substrate consumers are also the top-affected by the kernel
change, so the churn overlaps.

## 12. Red flags during a rewrite

Rewrites fail in characteristic ways. If any of these appear, stop
and reassess:

- **Week-three decision paralysis.** Two months in, the team is
  still arguing about naming. Means the design isn't fixed; go back
  to the doc before coding.
- **"We'll clean this up later" commits.** Rewrite with shortcut
  becomes incremental with scaffolding. The original code is better
  than two layers of half-built.
- **Silent test regressions.** A rewrite that doesn't pass the
  recorded test parity from 6.3 is not done. Don't ship it.
- **Users reporting behavior changes the team didn't intend.** The
  test parity missed something. Investigate, don't handwave.

Each is a signal the rewrite is off-track. Catching early costs
days; catching late costs months.

## 13. Rewrite governance

For each from-scratch rewrite, the team needs:

- A single named **owner** accountable for landing.
- A **written before/after contract** listing every trait or type
  that changes, with migration notes.
- A **rollback plan** — feature flag, fork branch, or reversible
  commit sequence.
- A **bake period** before removing the old path (minimum 1 week
  for 2.2–2.4; 2 weeks for 2.1 and 2.5).
- **Two sign-offs** before merging the kernel rewrite — one from
  the kernel owner, one from the consumer most affected.

This feels heavyweight until the first bad rewrite wrecks a week of
shipping. Then it feels cheap.

## 14. Cross-references

- Incremental alternative is `06-refactoring-plan.md`.
- Each rewrite candidate points at the home doc that justifies it:
  2.1 (→ 02, 03, 04), 2.2 (→ 10, 14), 2.3 (→ 11, 12), 2.4 (→ 17 §5),
  2.5 (→ 11, 14).
- The dep-graph changes the rewrites enable are in `20-modularity-composability.md`.
- The consolidated timeline that merges the rewrite plan with the
  incremental plan and the UX work is in `35-consolidated-roadmap.md`.

--- END 21-from-scratch-redesigns.md ---

# Batch REF21 — From-scratch rewrite candidates across implementation-readiness

**Refinement source**: `tmp/refinements/21-from-scratch-redesigns.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- `docs/00-architecture/31-implementation-readiness-audit.md` — five rewrite candidates + sequencing.
- `docs/00-architecture/23-architectural-analysis-improvements.md` — cross-reference rewrite list.

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/21-from-scratch-redesigns.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.


## Required vocabulary (verify)

The verify step greps for: `rewrite|from.?scratch`

At least one changed file must mention it (case-insensitive) before the
batch is considered complete.

## Cross-references

Dependents — downstream refinements assuming this one has propagated
into docs: REF35

## Rules

Follow all rules in `context-pack/00-REFINEMENTS-RULES.md`:

- Only touch files under `docs/`. The verify scope gate fails the
  batch otherwise.
- Aggressive edit posture authorised: full-file rewrites allowed when the
  refinement's framing contradicts the existing doc.
- Retired terms (see `context-pack/02-TERMINOLOGY-TABLE.md`) only
  appear in lines explicitly marked retired/deprecated/historical/
  formerly/legacy.
- Substantive edits — no "TODO: rewrite later" placeholders.

## Done when

- Diff gate + scope gate + terminology gate + required-term gate all pass.
- Commit ready with message `refinements(REF21): From-scratch rewrite candidates across implementation-readiness`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
