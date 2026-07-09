# Refinements Batch REF06

Run id: run-20260416-221511
Attempt: 2
Model: gpt-5.4
Reasoning: high
Refinement source: /Users/will/dev/nunchi/roko/roko/tmp/refinements/06-refactoring-plan.md
Target docs (candidates): docs/00-architecture/INDEX.md docs/00-architecture/31-implementation-readiness-audit.md

## Previous attempt failure context

Terminology gate failed: retired terms present in changed files.

Recent log tail:
- The top-level docs index now points readers to the new refactor-plan chapter.

Retired disclaimers removed:
- None directly in this batch. This batch added the plan chapter and cross-references rather than rewriting glossary/legacy-disclaimer sections.

Cross-references added:
- Every touched doc now references `tmp/refinements/06-refactoring-plan.md`.
- The new chapter also points at [01-naming-and-glossary.md](/Users/will/dev/nunchi/roko/roko/.roko/worktrees/refinements-run-20260416-221511/docs/00-architecture/01-naming-and-glossary.md).

Verification:
- Confirmed required vocabulary (`Phase A`, `Phase B`, `Phase C`) is present.
- Ran `git diff --check` on the touched files.
- Kept edits within `docs/` only.

Follow-ups identified:
- The broader Phase A glossary/doc cleanup described by REF06 is not fully executed here; this batch only lands the canonical phased-plan chapter and pointers.
- There were already unrelated doc changes in the worktree ([06-synapse-traits.md](/Users/will/dev/nunchi/roko/roko/.roko/worktrees/refinements-run-20260416-221511/docs/00-architecture/06-synapse-traits.md), [23-architectural-analysis-improvements.md](/Users/will/dev/nunchi/roko/roko/.roko/worktrees/refinements-run-20260416-221511/docs/00-architecture/23-architectural-analysis-improvements.md)); I left them untouched.

Commit-ready message:
`refinements(REF06): Land refactoring-plan phases as a dedicated architecture sub-chapter`

=== Finished: 2026-04-16T22:33:16+02:00 ===
=== Duration: 5m 42s ===
=== Exit code: 0 ===
[verify] diff_gate: 6 changed path(s) under docs/
[verify] terminology: scanning 6 file(s)
[verify] terminology violation in docs/00-architecture/31-implementation-readiness-audit.md: 251:**Crate reality:** bardo-primitives has `HdcVector` (3 files, ~500 LOC, 18 tests). The knowledge store
[verify] terminology violation in docs/00-architecture/31-implementation-readiness-audit.md: 538:| bardo-runtime | 6 | ~900 | ~12 | Yes | **Stable** |
[verify] terminology violation in docs/00-architecture/31-implementation-readiness-audit.md: 539:| bardo-primitives | 3 | ~500 | 18 | Yes | **Stable** |
[verify] terminology violation in docs/00-architecture/31-implementation-readiness-audit.md: 57:| 06 | Neuro | 4 | 4 | 5 | 3 | 3 | 3 | **22/30** | High | (in roko-core/roko-golem) |
[verify] terminology violation in docs/00-architecture/31-implementation-readiness-audit.md: 60:| 09 | Daimon | 5 | 4 | 4 | 3 | 3 | 4 | **23/30** | Medium-High | (in roko-golem) |
[verify] terminology violation in docs/00-architecture/31-implementation-readiness-audit.md: 61:| 10 | Dreams | 4 | 4 | 5 | 2 | 4 | 4 | **23/30** | High | (in roko-golem) |
[verify] terminology violation in docs/00-architecture/31-implementation-readiness-audit.md: 252:types are spread across roko-core and roko-golem without consolidation.
[verify] terminology violation in docs/00-architecture/31-implementation-readiness-audit.md: 311:- Two parallel implementations (roko-daimon 569 LOC + roko-golem/daimon.rs 972 LOC) need consolidation
[verify] terminology violation in docs/00-architecture/31-implementation-readiness-audit.md: 316:**Crate reality:** Code split across roko-golem (scaffold) and roko-core affect types.
[verify] terminology violation in docs/00-architecture/31-implementation-readiness-audit.md: 337:**Crate reality:** DreamRunner exists in roko-golem (scaffold). Core loop works but computational
[verify] terminology violation in docs/00-architecture/31-implementation-readiness-audit.md: 544:| roko-golem | 7 | ~600 | 3 | **No** | Scaffold |
[verify] terminology violation in docs/00-architecture/31-implementation-readiness-audit.md: 549:- **Scaffold** (roko-golem, roko-chain): Phase 2+ placeholder code
[verify] terminology violation in docs/00-architecture/31-implementation-readiness-audit.md: 575:| G12 | Consolidate roko-daimon + roko-golem/daimon.rs | 09 | Medium | Prerequisite for daimon features |
[verify] terminology violation in docs/00-architecture/23-architectural-analysis-improvements.md: 113:| **Sumers et al. 2023 (CoALA)** | 5 memories + 3 action types | Roko's six operators subsume that decomposition |

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

Suggested parallel split for batch `REF06`:

- worker: add a refactor-plan chapter under `docs/00-architecture/` (pick a
  stable number that doesn't collide, e.g. `33-refactor-plan.md` or similar),
  mirroring the Phase A/B/C/D breakdown.
- worker: update `docs/00-architecture/INDEX.md` with the new chapter.
- worker: optionally update `docs/00-architecture/31-implementation-readiness-audit.md`
  to reference the refactor phases.

## Canonical refinement source

This is the verbatim refinement proposal that this batch must propagate
into `docs/`. Treat it as the authoritative source. Do not edit this
file; only edit docs under `docs/`.

--- BEGIN 06-refactoring-plan.md ---

# Refactoring Plan

> **TL;DR**: Three phases — Docs Alignment (1 week), Kernel Addition
> (2 weeks), Subsystem Migration (3–4 weeks). Each phase is
> independently mergeable and reversible. No existing functionality
> breaks until Phase 3, and even then only via compiler-assisted
> signature updates.

> **For first-time readers**: This is the "how, in what order, with what
> risk" doc. It assumes you've read 01 (critique), 02 (Pulse), 03 (Bus),
> 04 (operator generalization), and 05 (loop). If you're comparing this
> plan against the broader roadmap, see `35-consolidated-roadmap.md` for
> where Phase A/B/C/D of this plan sit among the other refinement
> sequencing (HDC, demurrage, plugin SPI, web UI, etc.).

## Phase A — Docs Alignment (1 week, doc-only, no code)

**Goal**: Update every foundational doc to the two-medium / two-fabric
framing. Remove stale "Signal = Engram" disclaimers. No runtime
changes.

### A.1 Kernel-crate docs (roko-core)

- Rewrite the doc comment at the top of `crates/roko-core/src/lib.rs`.
  New version: "Roko kernel — two mediums (Engram, Pulse), two
  fabrics (Substrate, Bus), six operators…". Preserve the "every
  capability is a trait implementation" line.
- Update module-level doc on `crates/roko-core/src/engram.rs`:
  Engram is the durable medium; Pulse is its ephemeral sibling;
  see `pulse.rs` (to be added in Phase B).

### A.2 Architecture chapter (docs/00-architecture/)

- **New** sub-doc: `02b-pulse-ephemeral-event.md` — the Pulse medium
  (copy substantial content from `02-engram-vs-pulse.md` in this
  folder).
- **New** sub-doc: `07b-bus-transport-fabric.md` — the Bus trait
  (copy from `03-bus-as-first-class.md`).
- **Rewrite**: `02-engram-data-type.md` — remove the
  "Signal vs Engram" disclaimer, clarify Engram's role as the
  durable medium. Cross-link to `02b`.
- **Rewrite**: `06-synapse-traits.md` — retitle "The Six Synapse
  Traits" → "Synapse Operators Over Two Fabrics". Update signatures
  per `04-operators-generalized.md`.
- **Rewrite**: `07-substrate-trait.md` — clarify it's one of two
  kernel fabrics (cross-link to `07b`).
- **Rewrite**: `09-universal-cognitive-loop.md` — seven-step loop
  per `05-loop-retold.md`.
- **Rewrite**: `12-five-layer-taxonomy.md` — add Bus to L0 alongside
  Substrate.
- **Update**: `INDEX.md` abstract — "one noun, six verbs" → "two
  mediums, two fabrics, six operators, five layers, three speeds,
  three cross-cuts".
- **Update**: `01-naming-and-glossary.md` — remove the
  "In Rust code it is `Signal`" language (stale), add `Pulse` +
  `Bus` + `Topic` entries.
- **Update**: `24-cross-section-integration-map.md` — the
  `EngineEventBus` proposal is no longer a proposal; it's the
  `Bus` trait. Reflag those items.
- **No change**: `23-architectural-analysis-improvements.md` —
  analysis doc, historical record. Add a footer: "Post-2026-04-16
  refinement resolved several items in §2.2 (Boundary Operations)
  and §3.2 (the roko-conductor → roko-learn violation)".

### A.3 Top-level docs

- **README.md**: Rewrite the "One noun, six verbs" paragraph (line
  118) to the two-mediums framing.
- **CLAUDE.md**: Replace line 59 ("1 noun (Signal) + 6 verb traits…")
  with the new phrase.
- **docs/INDEX.md**: Replace the "Signal / Engram Naming" section
  with a "Two Mediums" section.

### A.4 Status docs

- `docs/STATUS.md`: Add a row for `Bus` and `Pulse` under **Scaffold**
  (or wherever the Phase-B landing puts them).
- `tmp/ux-followup/11-execution-plan.md`: Insert the kernel-addition
  phase (Phase B of this plan) as a new batch prompt, T33.

### Effort — Phase A

~1 week for a single engineer. Pure documentation work, no runtime
risk. Can be done as a single PR or as seven small PRs (one per
doc).

## Phase B — Kernel Addition (2 weeks, additive code, non-breaking)

**Goal**: Add `Bus`, `Pulse`, `Topic`, `TopicFilter` to `roko-core`.
Ship `BroadcastBus` and `MemoryBus` in `roko-std`. No existing code
changes its behavior.

### B.1 roko-core additions

New modules in `crates/roko-core/src/`:

- `pulse.rs` — the `Pulse` struct, `PulseSource`, `TraceId`.
- `topic.rs` — `Topic` newtype, `TopicFilter` enum.
- Add `Bus` trait to `traits.rs` (alongside the existing six).
- Extend `Datum<'_>` enum to cover either medium.
- Add `BusReceiver`, `PolicyOutputs`.
- Add `GraduationPolicy` default impl (§3.1 of `02-engram-vs-pulse.md`).
- Add `Engram::to_pulse` and `Pulse::graduate` conversion methods.

Export everything from `lib.rs`.

### B.2 roko-std additions

- `crates/roko-std/src/bus/broadcast.rs` — wraps
  `tokio::sync::broadcast`, replay via ring buffer.
- `crates/roko-std/src/bus/memory.rs` — synchronous in-memory Bus
  for testing.

### B.3 roko-runtime simplification

- `crates/roko-runtime/src/event_bus.rs` — keep as-is but mark
  module deprecated with `#[deprecated = "use roko_core::Bus trait
  + roko_std::BroadcastBus"]` on the `EventBus` struct. Provide a
  `impl Bus for EventBus<Pulse>` shim.
- `crates/roko-runtime/src/lib.rs` — unchanged exports; add a
  re-export of `roko_core::{Bus, Pulse, Topic, TopicFilter}` for
  convenience.

### B.4 Test additions

- `crates/roko-core/tests/pulse_graduation.rs` — Pulse → Engram
  round-trip, conversion-law property tests.
- `crates/roko-std/tests/broadcast_bus.rs` — fan-out correctness,
  replay semantics, ring-wrap behavior.
- `crates/roko-core/tests/topic_filter.rs` — glob matching,
  boolean combinations.

### B.5 Test count

Target: +40–60 new tests across `roko-core` + `roko-std`. Total
workspace test count moves from ~4,508 to ~4,560.

### Effort — Phase B

~2 weeks for a single engineer. All additive; no existing behavior
changes. Can ship behind a feature flag (`--features bus-kernel`)
if extra caution is wanted, though I don't think it's necessary.

## Phase C — Subsystem Migration (3–4 weeks)

**Goal**: Port the subsystems that already use ad-hoc event enums to
the `Bus` + `Pulse` model. Fix the layer violation from doc 23.
Close the two P0 self-hosting blockers.

### C.1 Migration order (by dependency)

1. **roko-runtime** — replace `Envelope<E>` usages in callers with
   `Pulse`. Typed event enums become topic-strings + `Kind`. (Week 1.)
2. **roko-orchestrator** — `OrchestrationEvent` enum →
   `orchestration.*` topics. (Week 1.)
3. **roko-agent** — internal agent-to-agent events → `agent.*`
   topics. WebSocket sidecar in `roko-agent-server` → publishes
   Pulses instead of an ad-hoc JSON frame type. (Week 2.)
4. **roko-conductor** — remove the `roko-learn` dependency.
   Conductor subscribes to `gate.verdict.emitted` and
   `gate.failure.rate`. This closes doc-23 violation. (Week 2.)
5. **roko-learn** — `CircuitBreakerPolicy` publishes
   `gate.failure.rate` Pulses. `EfficiencyPolicy` publishes
   `efficiency.tick` Pulses. `EpisodePolicy` subscribes to
   `substrate.engram.stored`. (Week 3.)
6. **roko-cli** TUI — replace the polling code paths flagged in
   `tmp/ux-followup/12-tui-event-parity.md` with Bus subscriptions.
   Two P0 bugs close. (Week 3.)
7. **roko-serve** — WebSocket/SSE endpoints expose Bus topics to
   HTTP consumers. Replace internal broadcast channels with Bus
   subscriptions. (Week 4.)
8. **Self-hosting closure** — implement `PlanRevisionPolicy` and
   `PrdPublishPolicy`. Closes CLAUDE.md items 10 and 11. (Week 4.)

### C.2 Per-subsystem migration recipe

For each subsystem the pattern is the same:

1. Identify the ad-hoc event enum (`AgentEvent`, `OrchestrationEvent`,
   …) currently on a `tokio::sync::broadcast` channel.
2. For each variant, pick a topic string and a `Kind` + `Body` shape.
   Record the mapping in `roko-core::topics::<subsystem>` as `const`
   declarations.
3. Replace `tx.send(AgentEvent::Foo(...))` with
   `bus.publish(Pulse { topic: TOPIC_AGENT_FOO, kind: Kind::X,
   body: Body::Json(...), ... })`.
4. Replace `while let Ok(evt) = rx.recv().await { match evt { ... }}`
   with `while let Some(pulse) = receiver.next().await { match
   pulse.kind { ... }}`.
5. Delete the old enum. Run tests.
6. Check call sites across the workspace and fix.

### C.3 Breaking changes

Minimal external surface changes:
- Public types `OrchestrationEvent`, `AgentEvent`, etc. removed. These
  were not part of any stable API.
- The `EventBus<E>` shim stays for one release with deprecation warnings.

### Effort — Phase C

~3–4 weeks with 1–2 engineers. Some weeks parallelizable:
orchestrator (week 1) and agent-server (week 2) are independent.
Conductor (week 2) depends on learn publishing (week 3) in the other
direction than you'd think — actually conductor migrates before
learn, with a stub that publishes to itself, then learn lands and
becomes the real publisher.

## Phase D — Chain & Mesh Buses (Phase 2+, when chain lands)

**Goal**: Bus has multiple backend implementations, not just
broadcast.

- `ChainBus` in `roko-chain` — `chain.*` topics map to on-chain
  events; replay maps to block scanning.
- `NatsBus` in `roko-mesh` (new crate) — for multi-process
  deployments.
- `MultiBus` in `roko-core` — composes several Bus backends.

This is post-Phase-C and depends on `roko-chain` and `roko-mesh`
landing as first-class crates, which is Tier 6 in
`tmp/MASTER-PLAN.md`.

## Total effort

| Phase | Scope | Engineers | Duration |
|---|---|---|---|
| A | Docs alignment | 1 | 1 week |
| B | Kernel addition | 1 | 2 weeks |
| C | Subsystem migration | 1–2 | 3–4 weeks |
| D | Chain & mesh buses | 1–2 | Phase 2+ |
| **Total (A–C)** | | 1–2 | **~6–7 weeks** |

## Rollback plan

- Phase A — revert the documentation PRs. No runtime effect.
- Phase B — revert the kernel-addition PRs. The added types are not
  yet used anywhere critical.
- Phase C — each subsystem migration is an independent PR. Revert
  only the affected subsystem.

There is no "point of no return" inside Phases A–C. Phase D is
reversible only within the crate being extended.

## Risks

1. **Pulse ring-buffer sizing.** Too small and subscribers lose data;
   too large and memory grows. Default 4096 per bus with per-topic
   overrides; monitor ring-occupancy Pulse (`bus.ring.occupancy`).
2. **Graduation policy regressions.** A Pulse that should have
   graduated but didn't is a forensic gap. Mitigate with a
   `graduation.missed` metric Pulse emitted by the Substrate when it
   sees a `Pulse with lineage_hint` but has no matching Engram.
3. **Schema drift across Bus backends.** When chain/mesh buses land,
   a Pulse published on broadcast-bus and re-published on chain-bus
   must be the same Pulse. Canonical encoding spec (probably
   CBOR over the Pulse struct's serde impl) prevents this.
4. **Doc drift during migration.** Phase A can land before Phase B.
   If Phase B slips, the docs describe a Bus that doesn't exist.
   Mitigate by adding a "Planned" banner to the Bus docs until Phase
   B lands.

## Dependencies on existing work

None. This refactor does not block and is not blocked by the items in
`tmp/ux-followup/`. Running them in parallel is safe because:

- The gap-catalogue items work inside the current trait signatures.
- The refactor generalizes the signatures without removing them.
- Both converge on the same subsystem (e.g. TUI event parity) but
  from different angles — ux-followup fixes the polling-vs-streaming
  bug with whatever mechanism is at hand; the refactor lets the
  fix be "subscribe to a Bus topic" rather than ad-hoc.

If the refactor lands first, the ux-followup items get simpler. If
ux-followup lands first, the refactor's migration scope shrinks.
Either order works.

## Checkpoint criteria

For each phase, concrete "phase is done" definitions the team can
check rather than debate.

### Phase A done when

- Every doc under `docs/00-architecture/` that mentions "one noun" has
  been updated to "two mediums."
- `docs/00-architecture/02-engram-data-type.md` and
  `docs/00-architecture/07-substrate-trait.md` are renamed to
  `02a-*` and `07a-*` with redirects.
- `02b-pulse-ephemeral-event.md` and `07b-bus-transport-fabric.md`
  exist with a "Planned" banner.
- `CLAUDE.md`, `README.md`, and `docs/INDEX.md` no longer carry
  "Signal = Engram" disclaimers.
- A single PR against `docs/00-architecture/23-architectural-analysis-improvements.md`
  adds a footer noting which items the refactor dissolved.

### Phase B done when

- `cargo build --workspace` succeeds with `Pulse`, `Bus`, `Topic`,
  `TopicFilter`, `Datum`, `PolicyOutputs` exported from `roko-core`.
- `BroadcastBus` and `MemoryBus` in `roko-std` have >90% line coverage
  in their own test modules.
- `Pulse::graduate` round-trip property tests green on 10k random
  Pulses.
- Topic-filter glob matcher passes a hand-written spec of 30+ cases.
- `Envelope<E>` carries a `#[deprecated]` attribute but existing
  callers still compile.
- No subsystem migration has started yet. If anyone touched
  `roko-conductor` or `roko-learn` during Phase B, they rolled it
  back.

### Phase C done when

- No call site in the workspace sends to a subsystem-specific event
  enum. `rg 'enum (Orchestration|Agent|Conductor|Ui)Event' crates/`
  returns zero hits outside `#[deprecated]` shims.
- `roko-conductor`'s Cargo.toml has no `roko-learn` dependency.
- `PlanRevisionPolicy` and `PrdPublishPolicy` ship as separate
  modules with integration tests that fake `gate.verdict.emitted`
  streams.
- The TUI polling-vs-streaming bugs in
  `tmp/ux-followup/12-tui-event-parity.md` have been moved to
  DONE with linked PRs.
- `roko-serve` WebSocket/SSE endpoints forward Bus subscriptions
  via `27-realtime-event-surface.md`'s wire protocol.
- The `#[deprecated]` `Envelope<E>` has been removed.

### Phase D done when

- `ChainBus` in `roko-chain` has parity with `BroadcastBus` on the
  trait surface and has one integration test that emits a
  Pulse-from-chain-event.
- `NatsBus` in `roko-mesh` has parity and one integration test.
- `MultiBus` in `roko-core` has a test that composes `BroadcastBus`
  and an in-memory test bus.

## Metrics that should move

A refactor with no measurable effect is indistinguishable from
shuffling deck chairs. These are the numbers to track across the
phases:

| Metric | Baseline | Target after Phase C |
|---|---|---|
| Cross-crate type imports (grep `pub use .*::(Orchestration|Agent|Conductor)Event`) | N hits | 0 |
| Polling loops in TUI (`loop { sleep … query }`) | ≥2 confirmed | 0 |
| `roko-conductor` Cargo dependencies | includes `roko-learn` | excludes `roko-learn` |
| `Policy::decide(&[], ctx)` call sites | ≥3 | 0 |
| P0 bugs in `tmp/ux-followup/12-tui-event-parity.md` | 2 | 0 |
| Workspace test count | ~4,508 | ≥4,560 |
| TUI event latency p95 (poll period → delivery) | ~250 ms (poll) | <20 ms (sub) |

The last row is the user-visible win — the TUI stops feeling laggy
on token-heavy turns.

## What comes after Phase C

Every subsequent refinement in this folder presumes the kernel has
landed. Specifically:

- `10-self-learning-cybernetic-loops.md` §9 (the "Phase C.5" that
  adds `prediction.*` / `outcome.*` topics) runs right after C.
- `11-hyperdimensional-substrate.md` §11.1 (fingerprint on every
  Engram) can run in parallel with C if a second engineer is
  available.
- `12-knowledge-demurrage.md` §10 (migration path) runs after C.
- `17-plugin-extension-architecture.md` Stage A (tier-3 tool
  manifests) runs after C.
- `26-statehub-rearchitecture.md` §11 (promote StateHub to a kernel
  crate) runs after C — it depends on the new Bus trait surface.

`35-consolidated-roadmap.md` draws the full dependency graph across
all 30+ refinement docs and suggests a six-to-twelve-month landing
sequence.

--- END 06-refactoring-plan.md ---

# Batch REF06 — Land refactoring-plan phases as a dedicated architecture sub-chapter

**Refinement source**: `tmp/refinements/06-refactoring-plan.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- Pick a non-colliding number in `docs/00-architecture/` (e.g. `33-refactor-plan.md`) for a new chapter that documents Phase A/B/C/D.
- `docs/00-architecture/INDEX.md` — add the new chapter.
- `docs/00-architecture/31-implementation-readiness-audit.md` — cross-reference the phased plan.

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/06-refactoring-plan.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.


## Required vocabulary (verify)

The verify step greps for: `Phase A|Phase B|Phase C`

At least one changed file must mention it (case-insensitive) before the
batch is considered complete.

## Cross-references

Dependents — downstream refinements assuming this one has propagated
into docs: REF21, REF35

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
- Commit ready with message `refinements(REF06): Land refactoring-plan phases as a dedicated architecture sub-chapter`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
