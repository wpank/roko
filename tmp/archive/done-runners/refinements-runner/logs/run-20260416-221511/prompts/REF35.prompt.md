# Refinements Batch REF35

Run id: run-20260416-221511
Attempt: 1
Model: gpt-5.4
Reasoning: high
Refinement source: /Users/will/dev/nunchi/roko/roko/tmp/refinements/35-consolidated-roadmap.md
Target docs (candidates): docs/00-architecture/31-implementation-readiness-audit.md docs/00-architecture/INDEX.md docs/INDEX.md

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

Suggested parallel split for batch `REF35`:

- worker: add a consolidated-roadmap chapter under `docs/00-architecture/`
  that sequences the refinements into a multi-quarter plan.
- worker: update `docs/00-architecture/INDEX.md` with the new chapter.
- worker: update `docs/INDEX.md` if a top-level roadmap pointer is needed.

## Canonical refinement source

This is the verbatim refinement proposal that this batch must propagate
into `docs/`. Treat it as the authoritative source. Do not edit this
file; only edit docs under `docs/`.

--- BEGIN 35-consolidated-roadmap.md ---

# Consolidated Roadmap

> **TL;DR**: The 30 previous refinement docs propose dozens of
> individual workstreams. Landing them in the wrong order produces
> wasted effort (doing HDC before demurrage before heuristics
> means redoing all three). This doc sequences the work into a
> six-to-twelve-month roadmap with explicit dependencies, effort
> estimates, and milestones. It is the single answer to "what do
> we build next?" across every other doc in this folder.

> **For first-time readers**: This doc is the sequencing layer. Read
> the individual refinement docs for the *why* and *how*; read this
> for the *when* and *in what order*. Every item cites its home doc
> so you can drill in. Nothing here is net-new design; it's all
> assembled from 02–34.

## 1. Design principles of the sequencing

- **Dependency order first.** A refinement whose substrate assumes
  another refinement's primitive lands after it, not before.
- **Risk budget at each phase.** No phase has more than one "high-
  risk" workstream (kernel rewrite, demurrage rate-tuning,
  multi-tenancy split). Risk stacks poorly.
- **Ship a user-visible win per quarter.** Long silent refactors
  kill morale. Each quarter closes with a demo-able result.
- **Parallelize the independent.** Foundation, learning, UX, and
  ecosystem tracks run in parallel when crate deps allow.
- **Non-blocking for ux-followup.** The refinements don't block or
  require the `tmp/ux-followup/` P0/P1 items; both advance on
  separate timelines.

## 2. The dependency graph

Simplified — nodes are refinement numbers; arrows mean "must land
after." Dashed arrows mean "benefits from but doesn't require."

```
01 critique ──▶ 02 Pulse ──▶ 03 Bus ──▶ 04 operators ──▶ 05 loop
                  │              │
                  ▼              ▼
                06 plan ────▶ 07 naming ──▶ 08 code
                              │
                              ▼
                            20 modularity
                              │
                              ▼
                 ┌──────────┐ │ ┌────────────┐ ┌────────────┐
                 │11 HDC    │◀┼─┤12 demurr.  │ │14 heurist. │
                 └──────────┘ │ └────────────┘ └────────────┘
                      │       │       │              │
                      ▼       ▼       ▼              ▼
                    10 self-learning ◀────────── 16 research
                              │
                              ▼
                    13 c-factor
                              │
                              ▼
                    15 scaling ◀─── 17 plugins ◀── 25 domains
                              │
                              ▼
                    18 moat, 19 catalog, 21 rewrites
                              │
                              ▼
                 ┌──────────┐ ┌──────────┐ ┌──────────┐
                 │26 State  │ │27 realtime│ │22 dev UX │
                 │  Hub     │ │  surface │ │          │
                 └──────────┘ └──────────┘ └──────────┘
                      │           │            │
                      ▼           ▼            ▼
                    23 user UX, 28 CLI, 29 web UI, 30 primitives
                      │           │            │
                      ▼           ▼            ▼
                    24 deployment
                      │
                      ▼
                    32 safety, 33 observability
                      │
                      ▼
                    09 phase-2, chain+mesh

                  31 synergy ── 34 glossary ── (orthogonal)
```

Refinements 01, 02, 03, 04, 05, 06, 07, 08 form the critical path:
nothing else ships without them. Refinements 20 (modularity) and 22
(dev UX) gate most of the ecosystem work.

## 3. Six-to-twelve-month roadmap by quarter

Each quarter names its headline deliverable — the user-visible win —
and the supporting tracks.

### Quarter 1 — Foundation

**Headline**: The two-medium kernel ships. Existing subsystems migrate
off ad-hoc event enums. Two P0 self-hosting closures land.

Tracks:

- **Foundation / kernel** (docs 01–09): Phase A + B + C of
  `06-refactoring-plan.md`. Pulse, Bus trait, Datum, operator
  generalization, seven-step loop, conductor migration,
  self-hosting PlanRevisionPolicy + PrdPublishPolicy.
- **Modularity** (doc 20): Extract `roko-bus` crate; scaffold
  `roko-spi`; CI enforcement of dep graph rules.
- **Naming & glossary** (docs 07, 34): Doc-level rename pass; the
  glossary lands alongside.
- **Observability** (doc 33 §17.1–§17.2): Roko-specific metrics
  wired; default Grafana dashboards shipped.

Risk: kernel refactor (high). Mitigations: feature-flag in 06 §6.1;
test parity before/after.

Demo: `roko plan run` on a PRD that's auto-generated from a published
PRD idea. No human touches the plan step.

### Quarter 2 — Learning substrate

**Headline**: HDC fingerprints everywhere, demurrage shipping,
heuristics becoming a real library, c-factor visible in dashboards.

Tracks:

- **HDC on every Engram** (11): field added; default encoder
  registered; `query_similar` on FileSubstrate.
- **Demurrage** (12): balance/reinforcement; cold tier; dashboard
  tile.
- **Heuristics** (14): type + Calibrator + CLI surface.
- **Self-learning** (10 §9): prediction/outcome topics;
  CalibrationPolicy; TUI F4 tab updates.
- **c-factor measurement** (13 §10 steps 1–2): metrics, dashboard
  tile. No Policy-level actuation yet.
- **Research-to-runtime** (16 §12 steps 1–3): Paper + Claim
  Engrams; starter kit of 20 papers.

Risk: demurrage rate-tuning (high). Mitigations: per-deployment
overrides; sliding-window CI to detect cold-tier blow-up;
opt-out in roko.toml.

Demo: `roko heuristic list` shows calibrated starter library; the
web UI Beliefs page renders it; c-factor gauge moves in real time
on a two-agent plan.

### Quarter 3 — Ecosystem and UX

**Headline**: Plugins are installable. StateHub is kernel-tier.
Realtime wire surface speaks a stable protocol. Web UI first
release. CLI parity with Claude Code shipping.

Tracks:

- **Plugin SPI** (17): Stage A + B + C — tier-3 tool manifests,
  tier-1/2 prompt/profile plugins, tier-4 ABI bridge.
- **StateHub rearchitecture** (26): kernel crate; canonical
  projections; in-process API; tests.
- **Realtime event surface** (27): WebSocket + SSE; wire protocol
  v1 frozen; TypeScript + Python + Rust clients.
- **Developer UX** (22): one-liner + builder API; four-layer SDK
  docs; `examples/` directory.
- **User UX** (23): interactive `roko init`; unified verb set; TUI
  goes interactive.
- **CLI parity** (28): slash commands; diff-first output; transcripts
  + resumption; budget visibility.
- **Web UI** (29): Home + Chat pages shipping; Plans + Beliefs in
  beta.
- **Rich UX primitives** (30): token streams; tool banners; gate
  badges; heuristic footnotes.
- **Deployment UX** (24): state export/import; Docker + Compose;
  single-server profile.

Risk: web UI scope creep (medium). Mitigations: strict five-page
cap; shadcn + Tailwind for speed; no custom design system.

Demo: an external developer ships a tier-3 tool plugin in one
afternoon and sees it running in the TUI, CLI, and web UI
simultaneously.

### Quarter 4 — Scale, safety, domains

**Headline**: Six domain profiles shipping. Safety spine visible
across every surface. Multi-tenant deployment model. Helm chart.

Tracks:

- **Domain profiles** (25): coding, research, blockchain, data, ops,
  writing. Each ships TypedContext schema + starter heuristics
  + gates + profile manifest.
- **Safety spine** (32): custody records for destructive actions;
  tier-5 WASM host; audit CLI.
- **Replication ledger** (16 §12 steps 4–7): ledger + export;
  watchdogs; provenance injection in prompts.
- **Deployment maturation** (24): multi-tenancy; OIDC; Helm chart;
  state portability at scale.
- **c-factor actuation** (13 §10 steps 3–4): Policy acts on c when
  process variables drop; devil's advocate; outsider injection.
- **Scaling instrumentation** (15): KPI dashboards; anti-metric
  alerts; kill-switch CLIs.
- **Commons** (14 §10): cross-deployment heuristic import/export;
  signature-based trust gradient.

Risk: multi-tenancy scope (high). Mitigations: start with
tenant-scoped Substrate namespace + manual auth; OIDC in a second
step.

Demo: a blockchain team installs the blockchain profile, runs a
simulated chain op with custody records and audit trail; an
observer in the web UI sees c-factor, cost, safety events in real
time.

### Quarter 5–6 (optional / Phase 2)

**Headline**: The chain, mesh, dreams layers come online. The
replication ledger crosses deployments. Roko starts feeding back its
own meta-research.

Tracks:

- **Phase 2 Bus/Substrate backends** (09): ChainBus, NatsBus,
  MultiBus.
- **Dreams cycle** (09 §2): Delta-speed consolidation with
  HDC-cluster-driven promotion.
- **Chain witnesses** (32 §8): attestation to on-chain; replication
  ledger cross-deployment trust.
- **Composer rewrite** (21 §2.5): query-driven templates; HDC-picked
  prompt parts.
- **Plugin registry** (17 Stage E): published plugin catalog with
  signed manifests and replication-ledger reputation.
- **Prediction markets** (15 §5.1): intra-system stake tokens for
  heuristic outcome betting.

These are explicitly Q5–Q6 items; Q1–Q4 stand on their own.

## 4. Parallelism and team shape

Minimum team to land Q1–Q4 in 12 months:

- **Kernel engineer (1)**: owns Q1; continues as steward through Q4.
- **Learning engineer (1–2)**: owns Q2 tracks 10, 11, 12, 14, 16.
  Continues into Q3–Q4 for polish.
- **UX engineer (2)**: owns Q3 tracks 22, 23, 28, 29, 30. Q4 polish
  + mobile.
- **Platform / deployment engineer (1)**: owns 17, 24, 27, 32, 33.
  Continues through Q4.
- **Domain lead (1, rotating)**: owns Q4 domain profiles. Domain-
  expert partnerships helpful.
- **Research lead (0.5)**: owns 16 starter kit curation; active
  oversight in Q2.

Total: 5–7 engineers for 12 months to land Q1–Q4 comfortably. With
fewer, drop domain profiles to 3 (coding, research, ops) and extend
by a quarter.

## 5. Risk register

Top risks across the whole roadmap, with mitigations:

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Kernel refactor breaks subsystems | Medium | High | Feature-flag; test parity; phase-B bake period |
| Demurrage rates over/under-tuned | High | Medium | Dashboard tiles; auto-tuning policy; kill-switch |
| HDC encoder drift across deploys | Medium | Medium | Versioned encoder; refuse cross-version mixing |
| Plugin ABI churn | Medium | High | Frozen ABI at each release; semver-strict |
| Web UI scope creep | High | Medium | Strict 5-page cap; no custom design system |
| Multi-tenancy auth complexity | Medium | High | Namespace-only in Q1; OIDC in Q2 |
| c-factor gets reward-hacked | Medium | Medium | c as covariate, not objective (13 §13) |
| Commons pollution | Medium | High | Curation; reputation; signature-based trust |
| Cross-doc consistency drift | High | Low | Glossary 34 is source of truth; CI checks |
| User confusion from new vocabulary | Medium | Medium | "For first-time readers" blocks in each doc; 34 |

Review the register quarterly; delete resolved, add new.

## 6. Decision checkpoints

Some refinements are "go/no-go" decisions, not continuous work.
Schedule explicit checkpoints:

- **After Q1 week 4**: Does the kernel refactor feel safe to continue?
  If not, revert to incremental patching; rescope.
- **After Q2**: Is demurrage reinforcement producing observable
  compounding (via the KPIs in 15 §10)? If not, tune or disable.
- **After Q3**: Is plugin ecosystem actually attracting plugins? If
  <5 external plugins in 6 weeks, audit onboarding.
- **After Q4**: Is any domain profile's replication ledger producing
  surprising findings? If yes, publish; if no, extend observation.

Each checkpoint has an owner and a written go/no-go. "Momentum"
isn't a checkpoint; evidence is.

## 7. Mapping to the existing catalog

Cross-reference to the current `tmp/ux-followup/` gap catalog:

| ux-followup section | Refinement equivalent | When it closes |
|---|---|---|
| 02 high-impact-quick-wins | Mostly polish; parallelized across Q1–Q3 | Q3 |
| 07 spec-code-drift (P0s) | 05 loop rewrite, 07 naming, doc rewrites | Q1 Phase A |
| 12 tui-event-parity (P0s) | Subsystem migration in 06 Phase C | Q1 end |
| 15 safety-and-learning-closure (P0s) | PlanRevisionPolicy + PrdPublishPolicy | Q1 end |
| 04 t9-t19-residuals | Policy migration to Bus subscriptions | Q1 Phase C |
| 05 partially-wired | Graduate to full wiring during Phase C | Q1 |
| 13 session-state-mgmt | State export/import in 24 | Q3 |
| 14 observability-gaps | 33 observability doc | Q2–Q3 |

Several ux-followup items become trivial once the refinements land
(the P0s in 12 are "subscribe to Bus topic"). Others (09
hygiene-and-test-coverage) are parallel hygiene work that doesn't
block either track.

## 8. Mapping to MASTER-PLAN tiers

The MASTER-PLAN.md tiers vs. refinements:

| Tier | MASTER-PLAN scope | Refinement coverage |
|---|---|---|
| 1 Mori parity | ~129 items | Closes gradually across Q1–Q3 |
| 2 Agent platform | ~81 items | Q3 (plugins, realtime) + Q4 (domains) |
| 3 Templates & events | ~28 items | Q2 (heuristics) + Q3 (realtime) |
| 4 Daemon & multi-repo | ~40 items | Q4 (deployment) |
| 5 Cognitive layer | ~92 items | Q2 (self-learning), Q5–Q6 (dreams) |
| 6 Chain layer | ~68 items | Q5–Q6 (Phase 2) |

The refinements don't replace MASTER-PLAN; they give it a framing
where each remaining item has a clearer "home doc." MASTER-PLAN
items should reference refinement numbers in their subsequent
updates.

## 9. Not-doing list

Explicit "we considered and rejected for now" items. The list is as
important as the roadmap; it says what we deliberately defer.

- **Custom LLM training on accumulated episodes**. Worth discussing
  later; not in scope now. The data compounds; training on it is a
  separate effort.
- **Graphical plan editor** (beyond the DAG view in web UI). The
  web UI surfaces plans but editing happens in the underlying
  plans/ markdown. A drag-and-drop editor is a Q6+ item.
- **Multi-language SDK** (Python / TS / Go native clients beyond
  what `27 §8` specifies). First-party clients for realtime is
  enough; full SDKs wait for demand.
- **Self-hosted LLM runtime**. Ollama/LM Studio are supported as
  backends (24 §1.1) but Roko doesn't ship its own inference server.
- **Kubernetes operator**. Helm chart (24 §7.1) is sufficient; an
  operator is a Q6+ item if demand materializes.
- **Mobile native app**. Progressive web app (29 §8) is sufficient;
  native mobile is a Q6+ consideration.
- **Voice-only interface**. Voice as an assist in chat (23 §5, 30
  §5) is fine; a standalone voice-driven workflow is out of scope.

Each of these can move onto the roadmap via an explicit proposal.
Until then, they're not.

## 10. One-year demo sequence

If Q1–Q4 ship, the one-year demo is a single 20-minute walkthrough:

1. `roko init` — interactive; user picks a profile.
2. `roko ask "what does this codebase do?"` — researcher role; web
   search turned on; result cites heuristics.
3. User asks to fix a bug. Roko classifies as multi-step, proposes
   plan. User approves.
4. Plan runs. Web UI shows live c-factor as two agents collaborate.
   Tool-call banners + reasoning stream visible. Gate badges green.
5. Heuristic is applied with a footnote; user hovers and sees
   calibration = 0.82 from 41 trials.
6. Plan completes. Diff shown per hunk. User accepts. Undo
   available.
7. User runs `roko custody list --after-plan <id>` — full chain of
   custody for every action.
8. User runs `roko cost report` — $0.42 spent; breakdown by model.
9. User installs a tier-3 tool plugin. Reruns. The plugin is in the
   toolset.
10. User exports session; imports on another machine; resumes.

Every step is a concrete capability from 02–30. If the demo works,
the roadmap worked.

## 11. Twelve-year view (aspirational)

Where 12 years of steady progress on this roadmap could lead:

- A deployed Roko runtime is an agent-research laboratory that
  publishes replication findings automatically.
- The heuristic commons rivals academic textbooks for engineering
  knowledge; chain-witnessed high-calibration heuristics are cited
  in papers.
- A dozen domains have well-developed profiles; mainstream companies
  deploy Roko for their core knowledge work.
- The substrate architecture is studied in graduate compilers
  courses (two-medium kernel, HDC-native content addressing,
  demurrage memory management).
- Plugin ecosystem: 10,000+ plugins; second-order plugins
  (plugin-of-plugins); cross-plugin commons emerge.
- Phase 2 on-chain witnessing creates a decentralized truth substrate
  for empirical engineering knowledge.

None of this is guaranteed. All of it becomes possible when Q1–Q4
actually ships and compounds. The near-term roadmap earns the right
to the long-term vision.

## 12. Cross-references

Every refinement doc is cited. In order:

- Foundation: 01, 02, 03, 04, 05, 06, 07, 08, 09.
- Learning: 10, 11, 12, 13, 14, 15, 16.
- Moat & ecosystem: 17, 18, 19, 20, 21.
- UX: 22, 23, 24, 25.
- Kernel UX plumbing: 26, 27.
- Surface UX: 28, 29, 30.
- Integrators: 31, 32, 33, 34, and this doc (35).

Plus the living planning material this roadmap intersects:
`tmp/MASTER-PLAN.md`, `tmp/ux-followup/00-INDEX.md`,
`docs/00-architecture/23-architectural-analysis-improvements.md`.

## 13. Maintenance

This doc is the single source of truth for sequencing.

- Updated at the end of each quarter with actual-vs-planned status.
- Checkpoint decisions (§6) logged in a changelog block at the
  bottom.
- Risk register (§5) reviewed quarterly.
- Not-doing list (§9) updated when items promote or demote.
- The dependency graph (§2) updates when refinements add or remove
  dependencies on each other.

## Changelog

- **2026-04-16** — Initial roadmap authored alongside refinements.
  Covers Q1–Q4 in earnest; Q5–Q6 as Phase-2 markers.

--- END 35-consolidated-roadmap.md ---

# Batch REF35 — Consolidated roadmap as architecture backmatter

**Refinement source**: `tmp/refinements/35-consolidated-roadmap.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- Add a consolidated roadmap chapter under `docs/00-architecture/` sequencing the refinements into Q1-Q4+.
- `docs/00-architecture/INDEX.md` — link new chapter.
- `docs/INDEX.md` — if a top-level roadmap pointer fits, add it.
- `docs/00-architecture/31-implementation-readiness-audit.md` — cross-reference.

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/35-consolidated-roadmap.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.


## Required vocabulary (verify)

The verify step greps for: `roadmap|Q1|Q4|sequencing`

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
- Commit ready with message `refinements(REF35): Consolidated roadmap as architecture backmatter`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
