# Refinements Batch REF34

Run id: run-20260416-221511
Attempt: 1
Model: gpt-5.4
Reasoning: high
Refinement source: /Users/will/dev/nunchi/roko/roko/tmp/refinements/34-glossary.md
Target docs (candidates): docs/00-architecture/01-naming-and-glossary.md docs/INDEX.md docs/00-architecture/INDEX.md

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

Suggested parallel split for batch `REF34`:

- worker: rewrite `docs/00-architecture/01-naming-and-glossary.md` to the
  full canonical glossary with a retired-terms table.
- worker: update `docs/INDEX.md` and `docs/00-architecture/INDEX.md` with
  glossary backrefs.

## Canonical refinement source

This is the verbatim refinement proposal that this batch must propagate
into `docs/`. Treat it as the authoritative source. Do not edit this
file; only edit docs under `docs/`.

--- BEGIN 34-glossary.md ---

# Glossary

> **TL;DR**: Every term introduced or reclaimed across the 33 earlier
> refinement docs, defined in one place. Entries are terse; each cites
> its home doc for depth. Use this as the canonical reference when
> writing new docs, code comments, or external content. If a term is
> missing here, it should be added before it ships to a wider
> audience.

> **For first-time readers**: This is an A–Z lookup. If you're reading
> other refinement docs and hit an unfamiliar term, search this one.
> The glossary is deliberately factual: definitions, not pitches.
> Cross-references point to the docs where each term earns its
> detailed treatment.

## Conventions

- Terms in **bold** at the start of each entry are the canonical form.
- `Code terms` use backticks.
- Cross-references use the convention from `00-INDEX.md`: bare
  filename for refinement docs, full path for existing architecture
  docs.
- A `(historical)` tag means the term appears in old code/docs and is
  being retired. `(new)` means the term is introduced in the
  refinements folder.
- Every entry cites a home doc. Where the term is split across
  multiple docs, the home is the one with the fullest treatment.

---

## A

**Active inference** — Free-Energy-Principle (Friston 2006) loop
implemented as predict-publish-correct Pulses across every operator.
See `10-self-learning-cybernetic-loops.md` §2.

**ACT** — Step 4 of the seven-step universal loop: execute the
composed Engram as an LLM call, tool call, or chain call. Produces a
stream of Pulses and typically a final AgentOutput Engram.
See `05-loop-retold.md` §3.

**Agent** — A running process or session that drives the universal
loop end-to-end. Historically Golem. See `01-naming-and-glossary.md`
in existing docs.

**Agent mesh** — Peer-to-peer layer for inter-agent coordination via
NATS/libp2p. Formerly Styx. Phase 2+. See `09-phase-2-implications.md`
§5, `24-deployment-ux.md` §1.4.

**Algedonic signal** — Cross-layer alarm that bypasses normal
hierarchy when a lower layer is failing (Beer's VSM term). Implemented
as a Bus Pulse on a priority topic.
See `10-self-learning-cybernetic-loops.md` §4.

**Annotation** — A typed human-authored Engram attached to a target
(episode, heuristic, plan, diff). Kinds: Note, Correction, Confirmation,
Question, Followup. See `30-rich-ux-primitives.md` §3.

**ASSESS** — Step 2 of the seven-step loop: joint Scorer + Router pass
that picks the next action and records confidence.
See `05-loop-retold.md` §3.

**Attestation** — Cryptographic signature over an Engram's content
hash. Levels: LocalAgent, OrgRole, ChainWitness.
See `32-safety-sandbox-provenance.md` §8.

**Authorization** — The `authorize(principal, action, target, ctx)`
function in `roko-agent/src/safety/`. Returns
Allow/Confirm/Once/Deny/Escalate. See `32-safety-sandbox-provenance.md` §2.

---

## B

**Balance** *(new)* — An Engram's demurrage-taxed attention credit.
Starts at 1.0; decays per time; restored by reinforcement.
See `12-knowledge-demurrage.md` §2.

**BROADCAST** — Step 6b of the seven-step loop (co-equal with
PERSIST): publish Pulses to the Bus. See `05-loop-retold.md` §3.

**Body** — Engram (or Pulse) payload. Variants: Text, Json, Bytes.
Reused between Engram and Pulse so graduation is identity.
See `02-engram-vs-pulse.md` §2.2.

**Bus** *(promoted)* — Kernel trait for transport of Pulses; sibling
to Substrate. Today a struct (`EventBus<E>`) in `roko-runtime`;
proposed to become a trait in `roko-core`.
See `03-bus-as-first-class.md`.

**`BusReceiver`** *(new)* — Handle returned by `Bus::subscribe`.
Yields Pulses in publish order. See `03-bus-as-first-class.md` §2.

**BroadcastBus** *(new)* — Default in-process Bus implementation
wrapping `tokio::sync::broadcast`. See `08-code-sketches.md` §4.

---

## C

**c-factor** — Woolley's collective intelligence factor, computed
continuously from Bus statistics for agent cohorts.
See `13-collective-intelligence-c-factor.md`.

**Calibrator** — Policy that updates a Heuristic's `Calibration` after
observing its predictions vs outcomes.
See `14-worldview-validation.md` §3.3, `10-self-learning-cybernetic-loops.md` §10.

**Calibration** — Per-heuristic (or per-claim, per-operator) record
of trials, confirmations, violations, Brier score, Wilson CI.
See `14-worldview-validation.md` §2.

**CascadeRouter** — Existing bandit-based model router in
`roko-learn/src/cascade_router.rs`. Picks model per turn.
See existing code; learning generalization in
`10-self-learning-cybernetic-loops.md` §7.1.

**ChainBus** *(Phase 2+)* — Bus backend that maps on-chain event logs
to Bus topics. See `09-phase-2-implications.md` §1.

**ChainSubstrate** — Substrate backend that persists attestations +
insights on-chain. Stubbed today; Phase 2+.

**Chain witness** — An Ed25519 (or similar) signature attesting to an
Engram's content, committed to a blockchain for cross-deployment
trust. See `32-safety-sandbox-provenance.md` §8,
`09-phase-2-implications.md` §1.

**Claim** *(new)* — Structured hypothesis derived from a Paper
Engram; includes falsifier, context, effect size, calibration.
See `16-research-to-runtime.md` §3.

**`claim!` macro** — Build-time macro resolving a ClaimId; produces
a runtime parameter that self-audits against the replication ledger.
See `16-research-to-runtime.md` §6.

**Cohort** — A set of agents working on a related task, sharing a
plan/PRD/parent episode. Unit of c-factor measurement.
See `13-collective-intelligence-c-factor.md` §2.1.

**Cold tier** — Substrate region for Engrams whose balance hit zero
under demurrage. Content retained but on slower storage; hash still
resolvable. See `12-knowledge-demurrage.md` §7.

**Commons** — Cross-deployment shared library of empirically-
validated heuristics. See `14-worldview-validation.md` §10,
`18-competitive-moat.md` §2.2.

**COMPOSE** — Step 3 of the seven-step loop; the Composer assembles
a prompt Engram under a budget. See `05-loop-retold.md` §3.

**Composer** — One of the six operators. Takes a slice of Datum,
produces an Engram (typically a Prompt). See `04-operators-generalized.md` §7.

**Consistency gate** — Stream-gate that detects semantic drift
between an agent output and its cited Engram support via HDC.
See `11-hyperdimensional-substrate.md` §8.

**ContentHash** — BLAKE3(kind + body + author + tags). Unique
identifier for Engrams. See `docs/00-architecture/02-engram-data-type.md`.

**Context** — Today: a struct passed to operator methods carrying
sidecar state (ctx id, run env, etc.). See existing code.

**Custody** *(new)* — Chain-of-custody record for an auditable
action: who, why, how, simulation, result, witness.
See `25-domain-specific-agents.md` §8.2, `32-safety-sandbox-provenance.md` §5.

---

## D

**Daimon** — Cross-cut handling affect (PAD vector); biases Scorer
and gates Actions. See `docs/00-architecture/13-cognitive-cross-cuts.md`,
`09-phase-2-implications.md` §9.

**`Datum`** *(new)* — Enum `Datum<'a> { Engram(&'a Engram), Pulse(&'a Pulse) }`
used by polymorphic operators. See `04-operators-generalized.md` §1,
`08-code-sketches.md` §3.

**Decay** — Engram decay curves: None, HalfLife, Ttl, Ebbinghaus.
Being superseded by Demurrage for attention weighting.
See `docs/00-architecture/05-decay.md`, `12-knowledge-demurrage.md` §10.

**Delta (1)** — Slowest cognitive speed (hours). Used by Dreams.
See `docs/00-architecture/10-heartbeat.md`.

**Delta (2)** — Incremental update to a StateHub projection's State.
See `26-statehub-rearchitecture.md` §3.

**Demurrage** *(new)* — Economic memory model: balance taxed per time,
restored by reinforcement. See `12-knowledge-demurrage.md`.

**Dissonance** — When two heuristics applicable to a situation would
predict different outcomes. A learning signal.
See `14-worldview-validation.md` §8.

**Domain** — One of Coding, Research, Blockchain, Data, Ops, Writing.
Each has a profile bundle of roles, tools, gates, heuristics.
See `25-domain-specific-agents.md`.

**Dreams** — Cross-cut; offline consolidation loop at Delta speed.
See `docs/00-architecture/10-dreams.md`, `09-phase-2-implications.md` §2.

---

## E

**Ebbinghaus** — A decay curve variant modeling forgetting curves.
See `docs/00-architecture/05-decay.md`.

**Engram** — Durable medium of Roko. Content-addressed, decayed,
scored, lineage-bearing record. Home type in `roko-core/src/engram.rs`.
See `02-engram-vs-pulse.md`.

**EngramBuilder** — Builder for constructing Engrams. Adds
fingerprint, lineage, score at build time. See existing code.

**Envelope** *(historical, `roko-runtime`)* — Wrapper around generic
event `E` carrying seq + timestamp. Being retired for `Pulse`.
See `02-engram-vs-pulse.md` §2, `07-naming.md` §9.

**Episode** — An Engram kind recording a full agent turn (inputs,
tool calls, output, verdicts). See existing code.

**Event** *(historical, avoided as a type name)* — Too generic;
retired in favor of `Pulse`. Still used colloquially in prose to mean
"something that happened."

**EventBus** *(historical, `roko-runtime`)* — Generic
typed broadcast channel. Being replaced by the `Bus` trait with
`Pulse` payload. See `03-bus-as-first-class.md` §6.

---

## F

**Fabric** — A kernel data-movement primitive. Roko has two:
Substrate (storage) and Bus (transport). See `03-bus-as-first-class.md` §1.

**Falsifier** — A Predicate attached to a Claim or Heuristic
specifying what observable would refute it.
See `14-worldview-validation.md` §2, `16-research-to-runtime.md` §13.

**Fingerprint** *(new)* — 10,240-bit HDC vector attached to every
Engram at put time; indexes similarity queries.
See `11-hyperdimensional-substrate.md` §3.

**Fleet** — Roster of agents (deployment-scoped). Formerly Clade.
See historical naming notes.

---

## G

**Gamma** — Fastest cognitive speed (5–15 s). Used by per-turn loops.
See `docs/00-architecture/10-heartbeat.md`.

**Gate** — One of six operators; verifies an Engram or Pulse window
against external truth. See `04-operators-generalized.md` §5.

**GateVerdict** — Engram kind produced by a Gate. Body includes pass/fail,
reason, evidence. See existing code.

**Golem** *(historical)* — Old name for Agent. Retired.

**Graduation** *(new)* — Converting a Pulse into an Engram for
durable persistence. Canonical path from transport to audit DAG.
See `02-engram-vs-pulse.md` §3, `08-code-sketches.md` §1.

**Grimoire** *(historical)* — Old name for Neuro. Retired.

---

## H

**Harness** — One of three deliverable surfaces (runtime, harness,
scaffold). Roughly: the scaffolding that runs agents with gates +
supervision. See `docs/00-architecture/12-five-layer-taxonomy.md` L3.

**HDC** — Hyperdimensional Computing (Kanerva 2009). 10,240-bit
binary-or-bipolar vectors; bind/bundle/permute algebra.
See `11-hyperdimensional-substrate.md`.

**`HdcVector`** — Rust type in `roko-primitives` / future
`roko-hdc` crate. See `11-hyperdimensional-substrate.md` §11.1.

**Heartbeat** — Cognitive clock publishing `heartbeat.gamma.tick`,
`heartbeat.theta.tick`, `heartbeat.delta.tick` Pulses.
See `09-phase-2-implications.md` §7.

**Heuristic** *(new)* — First-class Engram variant with
preconditions, prediction, calibration, lineage, receipts.
See `14-worldview-validation.md`.

**Holographic** — Property of HDC: partial information still
retrieves the whole; damage degrades gracefully.
See `11-hyperdimensional-substrate.md` §1.

---

## I

**Identity fingerprint** — HDC vector characterizing an agent's
recent Engrams; used for team discovery and diversity tracking.
See `11-hyperdimensional-substrate.md` §10.

**Intrinsic motivation** — Policy biasing attention toward high
prediction-error regions. See `10-self-learning-cybernetic-loops.md` §11.

---

## K

**Kernel** *(Roko-specific)* — The set of types and traits in
`roko-core` that every other crate depends on. Includes Engram,
Pulse, Substrate, Bus, Scorer, Gate, Router, Composer, Policy.
See `04-operators-generalized.md` §10.

**Kind** — Enum in `roko-core/src/kind.rs` enumerating semantic
categories of Engrams/Pulses (Plan, Task, GateVerdict, Episode,
Heuristic, Paper, etc.). ~28 variants today.

**Korai** — Agent-chain integration layer (blockchain). Formerly part
of Styx. Phase 2+. See `09-phase-2-implications.md` §1.

---

## L

**Layer (L0–L4)** — Five-layer taxonomy: Runtime, Framework, Scaffold,
Harness, Orchestration. Strictly downward dependencies.
See `docs/00-architecture/12-five-layer-taxonomy.md`.

**Lineage** — `Vec<ContentHash>` on an Engram pointing at its parents
in the audit DAG. See existing code.

**`loop_tick`** — The universal cognitive loop function in
`roko-core/src/loop_tick.rs`. Revised to 7 steps.
See `05-loop-retold.md` §8.

---

## M

**MCP** — Model Context Protocol. Standard for tool integration via
stdio/HTTP. Roko ships MCP integrations in `roko-mcp-*`.
See existing code; plugin-level story in `17-plugin-extension-architecture.md` §4.

**MetaGate** — Gate that runs on the agent's own self-model.
See `10-self-learning-cybernetic-loops.md` §6.3.

**Mesh** — See Agent mesh.

**MultiBus** *(new)* — Bus backend composing several Bus backends
behind one interface. See `03-bus-as-first-class.md` §3.2.

---

## N

**Neuro** — Cross-cut; durable knowledge store, distillation, tier
progression. Formerly Grimoire. See `crates/roko-neuro/`,
`docs/00-architecture/13-cognitive-cross-cuts.md`.

**Novelty** — `1 - max(similarity)` over top-K HDC neighbors; used by
demurrage reinforcement to weight uniqueness.
See `12-knowledge-demurrage.md` §3.

---

## O

**Operator** — One of the six kernel verb traits: Scorer, Gate,
Router, Composer, Policy (plus Substrate, Bus as fabric operators).
See `04-operators-generalized.md`.

**Orchestrator** — Layer-4 subsystem that runs plans, dispatches
tasks, enforces merge queues. See `crates/roko-orchestrator/`.

**Outcome Pulse** *(new)* — Pulse on `outcome.*` topic that closes
the loop on a previously-published prediction Pulse.
See `10-self-learning-cybernetic-loops.md` §2.2.

---

## P

**PAD vector** — Pleasure-Arousal-Dominance affective state;
maintained by Daimon. See `docs/00-architecture/09-daimon.md`.

**Paper** *(new)* — Engram kind representing an academic paper,
with DOI, authors, abstract, fingerprint, claims.
See `16-research-to-runtime.md` §2.

**PERSIST** — Step 6a of the seven-step loop; write an Engram to
Substrate. See `05-loop-retold.md` §3.

**Pheromone** — Engram kind used for stigmergic coordination between
agents. See `09-phase-2-implications.md` §3.

**Plan** — Engram kind representing a structured multi-task plan
with DAG edges. See existing code.

**Playbook** — Engram kind storing a distilled reusable action
sequence. See existing code; relationship to heuristics in
`14-worldview-validation.md` §1.

**Plugin** — Third-party extension. Five tiers of power/risk:
prompts, profiles, manifests, native, WASM.
See `17-plugin-extension-architecture.md`.

**Policy** — One of six operators; reacts to streams of Pulses,
emits new Pulses and Engrams. See `04-operators-generalized.md` §8.

**`PolicyOutputs`** *(new)* — Return type of `Policy::decide`;
contains `{ pulses, engrams }`. See `04-operators-generalized.md` §8.

**Prediction Pulse** *(new)* — Pulse on `prediction.*` topic
emitted by an operator when it makes a decision; matched to a
later `outcome.*` Pulse via lineage_hint.
See `10-self-learning-cybernetic-loops.md` §2.2.

**PRD** — Product Requirements Document. A directory in `.roko/prd/`
representing a work item's lifecycle (idea, draft, plan).

**Principal** — User, agent, or plugin; the subject of an
authorization decision. See `32-safety-sandbox-provenance.md` §2.

**Projection** *(new, StateHub)* — Named, typed, live-updating
view on the Bus + Substrate; has `State` and `Delta` types plus
a folding function. See `26-statehub-rearchitecture.md` §3.

**Profile** — A bundle of defaults: either a deployment profile
(laptop / single-server / container / ...) or a domain profile
(coding / research / blockchain / ...). Context disambiguates.
See `24-deployment-ux.md` §2, `25-domain-specific-agents.md` §9.

**Provenance** — Full author/trust/taint/attestation record on an
Engram. See existing code.

**Pulse** *(new)* — Ephemeral medium of Roko. Typed,
sequence-numbered, topic-addressed, ring-buffered message.
Lives on a Bus. Can graduate to Engram. See `02-engram-vs-pulse.md`.

**`PulseSource`** *(new)* — Light origin attribution struct on
every Pulse: `{ component, agent_id }`.
See `08-code-sketches.md` §1.

---

## Q

**`query_similar`** *(new)* — Substrate method returning Engrams
whose HDC fingerprint is within `radius` of a query fingerprint.
See `11-hyperdimensional-substrate.md` §4.

---

## R

**REACT** — Step 7 of the seven-step loop: Policy.decide produces
new Pulses + Engrams. See `05-loop-retold.md` §3.

**Reinforcement** *(new, demurrage)* — Bonus to an Engram's balance
when it's cited, retrieved, gated, surprises, or agent-quoted.
See `12-knowledge-demurrage.md` §2, §3.

**`ReinforceKind`** *(new)* — Enum: Cited, Retrieved, Gated,
Surprised, AgentQuoted. See `12-knowledge-demurrage.md` §2.

**Replication ledger** *(new)* — Per-claim record of
paper-reported-effect vs our-observed-effect, with CI and status.
See `16-research-to-runtime.md` §5.

**Role** — A composition template + tool allow-list + gate defaults.
Examples: researcher, planner, implementer, reviewer, compliance.
See `crates/roko-compose/src/templates/`.

**Router** — One of six operators; picks among candidates.
See `04-operators-generalized.md` §6.

**Runtime** — Layer-0 subsystem. Process supervisor, cancellation,
Bus, Substrate. See `crates/roko-runtime/`.

---

## S

**Scaffold** — One of three deliverable surfaces. Roughly: the
structure that agents compose within. See `docs/00-architecture/12-five-layer-taxonomy.md` L2.

**Score** — 7-axis appraisal attached to an Engram by the Scorer.
See `docs/00-architecture/04-seven-axis-score.md`.

**Scorer** — One of six operators; computes Score for any Datum.
See `04-operators-generalized.md` §4.

**SENSE** — Step 1 of the seven-step loop: perceive from three
sources (Substrate, Bus, external I/O). See `05-loop-retold.md` §3.

**Session** — A bounded run of agent interaction, typically
ephemeral unless graduated. Transcripts exportable.
See `23-user-ux-running-agents.md` §12.

**Signal** *(historical)* — Old name for Engram. Retired in 877:5
rename. See `07-naming.md` §2.2.

**SPI** — Service Provider Interface; the extension-point surface
for plugins. See `17-plugin-extension-architecture.md`.

**Stigmergy** — Indirect coordination via shared environment
(Grassé 1959). Implemented as Pheromone Engrams + `mesh.pheromone.*`
Pulses. See `09-phase-2-implications.md` §3.

**StateHub** *(promoted)* — Kernel projection layer. Today
TUI-specific; proposed to become a first-class kernel subsystem.
See `26-statehub-rearchitecture.md`.

**Styx** *(historical)* — Old name for Agent mesh. Split into
Mesh and Korai.

**Substrate** — Kernel trait for storage of Engrams.
See `docs/00-architecture/07-substrate-trait.md`, `03-bus-as-first-class.md` §1.

**Swarm** — Collective of agents subscribed to the same topic set;
outputs union across agents. See `09-phase-2-implications.md` §6.

---

## T

**Taint** — Metadata indicating untrusted input origin; propagates
through derived Engrams. See `32-safety-sandbox-provenance.md` §7.

**Theta** — Middle cognitive speed (~75 s). Used by the main plan-
execute loop. See `docs/00-architecture/10-heartbeat.md`.

**Topic** *(new)* — Routing handle for Bus publish/subscribe.
Dot-separated lowercase, e.g. `gate.verdict.emitted`. Type
`Topic(String)`. See `03-bus-as-first-class.md` §2.1,
`07-naming.md` §7.

**`TopicFilter`** *(new)* — Declarative filter for subscriptions.
Variants: Exact, Glob, AnyOf, All, And, Or, Not.
See `03-bus-as-first-class.md` §2.1.

**Trust score** — Per-agent-pair, per-topic accumulated reputation.
See `13-collective-intelligence-c-factor.md` §3.3.

**TypedContext** *(new)* — Structured domain situation data.
`{ domain, fields: BTreeMap<Key, Value> }`. Gates and heuristics
match typed predicates rather than free text.
See `25-domain-specific-agents.md` §8.1.

---

## U

**Undo** — Three-level mechanism: ephemeral (chat edit), short-term
(`roko undo last`), long-term (`roko replay ... --revert`).
See `23-user-ux-running-agents.md` §11.

**Universal loop** — Seven-step cognitive loop: SENSE, ASSESS,
COMPOSE, ACT, VERIFY, PERSIST/BROADCAST, REACT.
See `05-loop-retold.md`.

---

## V

**Verdict** — Output of a Gate. Always materialized as a
`GateVerdict` Engram so the audit DAG is preserved.
See `04-operators-generalized.md` §5.

**VERIFY** — Step 5 of the seven-step loop: Gate (or stream-gate)
verifies an Engram (or Pulse window) and emits a Verdict.
See `05-loop-retold.md` §3.

---

## W

**Watchdog** — Policy subscribed to a Claim's falsifier predicate
across all episodes; updates the replication ledger automatically.
See `16-research-to-runtime.md` §7.3.

**Wilson CI** — Wilson score interval for a binomial confidence
bound; used by Calibration. See `14-worldview-validation.md` §2.

**WisdomGate** — Gate enforcing Surowiecki's four conditions
(diversity, independence, decentralization, aggregation) before
a consensus Engram is finalized.
See `13-collective-intelligence-c-factor.md` §4.

**Worldview** *(new)* — Co-citation cluster of mutually-supporting
heuristics that dominate a domain-fingerprinted region of
situations. See `14-worldview-validation.md` §4.

**Witness** — See Chain witness.

---

## Retired / deprecated terms

These appear in historical code or docs; do not use in new work.

| Old | Replaced by | Reason |
|---|---|---|
| `Signal` (durable) | `Engram` | 877:5 rename already landed |
| `Signal` (ephemeral) | `Pulse` | Naming-cleanup cost too high to reclaim |
| `EventBus<E>` | `Bus` trait + `Pulse` | Ad-hoc generic; not canonical |
| `Envelope<E>` | `Pulse` | Implementation name leaked out |
| `Message` | `Pulse` (wire) / `ChatMessage` (LLM) | Ambiguous with LLM chat |
| `Event` | `Pulse` | Collides with every other framework |
| `Bardo`, `Golem`, `Mori` | `Roko` + `Agent` | Previous codename heritage |
| `Styx` | `Mesh` + `Korai` | Split into two clearer concepts |
| `Grimoire` | `Neuro` | Less mystical |
| `Clade` | `Fleet` | More conventional |
| `decay` (field on Engram) | `balance` + `demurrage` | Demurrage supersedes time-only decay |

## Terms deliberately not defined here

A few terms are used colloquially in the docs but aren't formal Roko
terms. Left to standard engineering usage:

- "session" (in casual sense, distinct from the formal Session
  above — colloquial uses always clarify context)
- "session" in the OIDC/HTTP sense (authentication session)
- "task" (agent's unit of work; vs. a background task)
- "model" (LLM, distinct from runtime model)
- "cost" (USD)

If any of these starts behaving as a technical term in a new doc,
promote them to this glossary.

## Maintenance

This doc is the canonical vocabulary. Rules:

- Every new technical term introduced in a refinement doc adds a
  glossary entry in the same PR.
- Retiring a term moves it to the "Retired" table with a reason.
- Cross-references in refinement docs use the glossary spellings.
- Annual review prunes entries that never took.

## Cross-references

- Historical naming decisions: `docs/00-architecture/01-naming-and-glossary.md`.
- Naming rationale for the new terms: `07-naming.md`.
- The synergy map that stitches primitives together: `31-synergy-integration-map.md`.

--- END 34-glossary.md ---

# Batch REF34 — Glossary consolidation across naming-and-glossary

**Refinement source**: `tmp/refinements/34-glossary.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- `docs/00-architecture/01-naming-and-glossary.md` — full canonical glossary with retired-terms table; A-Z vocabulary.
- `docs/INDEX.md` — glossary backref.
- `docs/00-architecture/INDEX.md` — glossary backref.

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/34-glossary.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.


## Required vocabulary (verify)

The verify step greps for: `glossary|retired`

At least one changed file must mention it (case-insensitive) before the
batch is considered complete.

## Cross-references

Dependents — downstream refinements assuming this one has propagated
into docs: (none)

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
- Commit ready with message `refinements(REF34): Glossary consolidation across naming-and-glossary`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
