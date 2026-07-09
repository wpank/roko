# Refinements Batch REF03

Run id: run-20260416-221511
Attempt: 1
Model: gpt-5.4
Reasoning: high
Refinement source: /Users/will/dev/nunchi/roko/roko/tmp/refinements/03-bus-as-first-class.md
Target docs (candidates): docs/00-architecture/07-substrate-trait.md docs/00-architecture/12-five-layer-taxonomy.md docs/00-architecture/24-cross-section-integration-map.md docs/00-architecture/INDEX.md

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

Suggested parallel split for batch `REF03`:

- worker: rewrite `docs/00-architecture/07-substrate-trait.md` to present both
  kernel fabrics; add `07b-bus-transport-fabric.md` (or substantial Bus section).
- worker: update `docs/00-architecture/12-five-layer-taxonomy.md` to list Bus
  at L0 alongside Substrate.
- worker: update `docs/00-architecture/24-cross-section-integration-map.md`
  so the EngineEventBus proposal points at the Bus trait.

## Canonical refinement source

This is the verbatim refinement proposal that this batch must propagate
into `docs/`. Treat it as the authoritative source. Do not edit this
file; only edit docs under `docs/`.

--- BEGIN 03-bus-as-first-class.md ---

# Bus as a First-Class Kernel Primitive

> **TL;DR**: Promote the event bus to a kernel trait in `roko-core`,
> paired with the existing `Substrate` trait. Substrate is the storage
> fabric; Bus is the transport fabric. Both are L0. Together they are
> the complete kernel of Roko's runtime.

> **For first-time readers**: Roko already has a `Substrate` trait in
> `crates/roko-core/src/traits.rs` (persist, query, prune durable Engrams)
> and an `EventBus<E>` struct in `crates/roko-runtime/src/event_bus.rs`
> (generic typed broadcast channel with replay ring). The Bus works, but it
> isn't in the architectural lexicon — no trait, no doc chapter, no stable
> name. This proposal promotes it: a `Bus` trait at kernel tier, `Pulse` as
> its payload (see 02), topics as routing handles, and a bounded ring as
> replay memory. Nothing in the current Bus implementation goes away — it
> becomes the default `BroadcastBus` implementation of the new trait.

## 1. The two fabrics

The current kernel presents one fabric — **storage** via the `Substrate`
trait at `crates/roko-core/src/traits.rs`. Every subsystem that needs
to communicate persistently uses it. Four backends already exist
(`MemorySubstrate`, `FileSubstrate`, `HdcSubstrate`, `ChainSubstrate`)
and they are API-identical from a caller's perspective.

The proposed kernel adds a second fabric — **transport** via a new
`Bus` trait. It already exists in spirit at
`crates/roko-runtime/src/event_bus.rs` as `EventBus<E>`. The refactor
canonicalizes its interface, moves it into `roko-core` at the same
layer as Substrate, and makes Pulse (not a user-defined enum) its
payload.

| | **Substrate** | **Bus** |
|---|---|---|
| Medium | Engram | Pulse |
| Shape | Put/Get/Query/Prune | Publish/Subscribe/Replay |
| Semantics | Idempotent content-addressed write; query by filter | Broadcast fan-out; topic-addressed; bounded ring for replay |
| Durability | Long-lived; decays over time | Brief; bounded by ring capacity |
| Concurrency | `Send + Sync`, handles concurrent puts/queries | `Send + Sync`, handles concurrent subscribers |
| Backends shipping today | Memory, File, (HDC built, Chain stubbed) | In-process broadcast (`tokio::sync::broadcast`) |
| Backends future | Any key-value or log store | NATS, Kafka, Redpanda, chain pubsub |
| Crate location | `roko-fs`, `roko-std`, `roko-neuro` impls | proposed: `roko-std` for broadcast, `roko-mesh` for NATS, `roko-chain` for chain pubsub |

## 2. Proposed trait

```rust
// crates/roko-core/src/traits.rs (new section)

use crate::{Pulse, Topic, TopicFilter, error::Result};
use async_trait::async_trait;

/// Transport fabric for Pulses.
///
/// A Bus delivers Pulses from publishers to subscribers. All Bus
/// implementations are API-identical from a caller's perspective — pick
/// the backend that matches your fan-out, durability, and latency needs.
///
/// # Delivery model
///
/// Bus is broadcast: every subscriber sees every Pulse on topics it
/// matches. There is no queuing or redelivery. Subscribers that fall
/// behind the ring buffer lose Pulses. For critical data, graduate
/// the Pulse to an Engram and subscribe to the Substrate.
///
/// # Replay
///
/// `replay_since(seq)` returns Pulses whose global sequence is
/// strictly greater than `seq` and still in the ring buffer. The
/// caller uses this to catch up after a brief disconnect or to
/// bootstrap a late subscriber.
///
/// # Concurrency
///
/// Buses are `Send + Sync`. Impls must handle concurrent publishers
/// and subscribers internally.
#[async_trait]
pub trait Bus: Send + Sync {
    /// Publish a Pulse. Returns its global sequence number.
    async fn publish(&self, pulse: Pulse) -> Result<u64>;

    /// Subscribe to a topic filter. Returns a receiver that yields
    /// Pulses in publish order. The receiver is cancel-safe.
    async fn subscribe(&self, filter: TopicFilter) -> Result<BusReceiver>;

    /// Replay Pulses newer than `since_seq` matching `filter`, up to
    /// the ring buffer's retention window. Used for resume after a
    /// brief disconnect.
    async fn replay_since(
        &self,
        since_seq: u64,
        filter: &TopicFilter,
    ) -> Result<Vec<Pulse>>;

    /// Current global sequence number (for checkpointing).
    async fn current_seq(&self) -> Result<u64>;

    /// Total Pulses published since bus start (for metrics).
    async fn total_published(&self) -> Result<u64>;

    /// Ring buffer current occupancy (for health checks).
    async fn ring_len(&self) -> Result<usize>;

    /// Ring buffer capacity (for health checks).
    fn ring_capacity(&self) -> usize;

    /// Human-readable name for logging/debugging.
    fn name(&self) -> &'static str {
        "unnamed_bus"
    }
}

/// A subscriber handle.
pub struct BusReceiver {
    pub inner: tokio::sync::mpsc::Receiver<Pulse>,
    pub last_seq: std::sync::atomic::AtomicU64,
}
```

### 2.1 TopicFilter

```rust
/// A declarative filter for Bus subscriptions.
pub enum TopicFilter {
    /// Match exactly one topic.
    Exact(Topic),
    /// Match a glob pattern, e.g. "agent.*" or "gate.verdict.*".
    Glob(String),
    /// Match any topic from the set.
    AnyOf(Vec<Topic>),
    /// Match all topics.
    All,
    /// Boolean AND.
    And(Box<TopicFilter>, Box<TopicFilter>),
    /// Boolean OR.
    Or(Box<TopicFilter>, Box<TopicFilter>),
    /// Boolean NOT.
    Not(Box<TopicFilter>),
}
```

## 3. Backends

### 3.1 Broadcast (in-process)

The default, shipping immediately. Wraps `tokio::sync::broadcast`. The
current `EventBus<E>` in `roko-runtime` becomes this, with the
signature simplified to take `Pulse` instead of a generic `E`.

### 3.2 MultiBus

Composes multiple Bus backends behind a single interface. Used by the
dashboard to see both in-process agent Pulses and incoming HTTP
webhook Pulses as one stream.

### 3.3 NATS / Redpanda / Kafka

For multi-process deployments (the `roko serve` control plane + remote
agent workers pattern described in `docs/VISION-RUN-ANYWHERE.md`).
Shipping this is post-Phase-1 but the trait supports it today.

### 3.4 ChainBus

Phase 2+. `chain.*` topics map to on-chain event logs. Subscribers
tail the chain via RPC. Replay maps to block scanning. Mirrors the
`ChainSubstrate` model.

### 3.5 MemoryBus

For testing. Drop-in for BroadcastBus without spawning a Tokio task.

## 4. Wiring: L0 Runtime becomes complete

Current L0 (per `docs/00-architecture/12-five-layer-taxonomy.md`):

> **Layer 0: Runtime** — Process lifecycle, event bus, supervision,
> cancellation, I/O, adaptive clock.
>
> Key Crates: `roko-primitives`, `roko-runtime`
>
> Synapse Traits at L0: `Substrate`

The doc already lists "event bus" as an L0 concern, but there's no
Synapse trait for it. After the refactor:

> Synapse Traits at L0: `Substrate`, `Bus`

And the two-fabric story becomes the kernel's executive summary:

> The Roko kernel is two fabrics — `Substrate` for durable Engrams and
> `Bus` for ephemeral Pulses. Every subsystem talks to the rest of
> Roko through one or both of these fabrics. Dependencies flow
> downward from higher layers to L0; higher-layer communication
> never bypasses the fabrics.

## 5. How this fixes doc 23's layer violation

`docs/00-architecture/23-architectural-analysis-improvements.md` §3.2
flagged exactly one confirmed violation: `roko-conductor → roko-learn`.
Root cause:

> `roko-conductor` imports learning types for circuit breaker state
> tracking. The Conductor needs to know about historical failure rates
> (a learning concern) to make circuit breaker decisions (a harness
> concern).

Doc 23's fix: extract a `HealthMetrics` trait into `roko-core` L0.
That works but adds a third trait surface.

**The Bus-first fix is simpler and subsumes it.** `roko-learn` already
emits gate-verdict-derived stats. Instead of `roko-conductor` calling
into `roko-learn` types, both subsystems subscribe to the same topic
family:

- `gate.verdict.emitted` — published by GatePipeline.
- `gate.failure.rate` — computed by `roko-learn`'s `CircuitBreakerPolicy`
  over a rolling window, published at some cadence.

`roko-conductor` subscribes to `gate.failure.rate` and reacts. No
compile-time dependency on `roko-learn`. Both crates depend only on
`roko-core` (which now owns `Bus` and `Pulse`).

This pattern generalizes. Every cross-layer coupling in the codebase
can be audited with the question "could this be a Bus topic instead?"
— and most of the time the answer is yes.

## 6. What `roko-runtime` becomes

Today `roko-runtime` owns:

- `event_bus` (becomes a Bus backend — move to `roko-std`)
- `process` (ProcessSupervisor — stays)
- `cancel` (cancellation tokens — stays)
- `metrics` (JSONL metric recording — consider moving to `roko-obs`)
- `resource` (limits, tracking — stays)

After the refactor:

- `roko-core` owns the `Bus` trait, `Pulse`, `Topic`, `TopicFilter`,
  `BusReceiver`, `GraduationPolicy`.
- `roko-std` owns `BroadcastBus` (the in-process impl) and `MemoryBus`.
- `roko-runtime` owns `ProcessSupervisor`, cancellation,
  resource-limit primitives. It depends on `roko-core::Bus` to publish
  process lifecycle Pulses.
- `roko-mesh` (new, Phase 2) owns `NatsBus`, `KafkaBus`.
- `roko-chain` extends to own `ChainBus` alongside `ChainSubstrate`.

## 7. Breaking change surface

Because today there's no `Bus` trait, there's nothing to break. The
refactor adds a trait, adds a type, and migrates internal call sites
from ad-hoc `EventBus<SomeEnum>` to `Bus` + `Pulse`.

The `Envelope<E>` type stays around as a deprecated alias for one
release so in-flight PRs aren't blocked. Then it's removed.

## 8. Open questions for this proposal

1. **Does `Bus` need its own `prune`?** Substrate has `prune` for
   decay-based eviction. Bus has ring-buffer eviction which is FIFO,
   not decay-aware. Probably don't need `prune` on `Bus`.
2. **Should the Bus carry `Context`?** Substrate methods take
   `&Context`. Bus publish doesn't naturally need it (the publisher
   knows its own context). Subscribe might benefit from it for
   authorization. Lean: don't add `Context` to Bus methods until we
   have a concrete authorization story — see `32-safety-sandbox-provenance.md`
   for where that story lands.
3. **Schema evolution for topics.** If a Pulse's Body shape changes,
   how do subscribers know? For now: reuse Engram's approach
   (non-exhaustive enums, `Custom(String)` escape hatch, `Body::Json`
   for structured). Formal topic schemas can wait for a v2.
4. **Wildcard unsubscribe.** If a subscriber uses `Glob("agent.*")`
   and a new topic `agent.heartbeat` is introduced, does the
   subscriber want it? Probably yes — documented behavior.
5. **Replay-window sizing.** 4096 Pulses is the default ring capacity.
   On a quiet system this is minutes of history. On a hot streaming
   agent, it can be under a second. Per-topic overrides plus an
   observable `bus.ring.occupancy` Pulse let operators spot the
   problem before data is lost. A cluster-scale Bus (NATS / Kafka)
   inherits that backend's retention policy, not ours.
6. **Ordering across topics.** Per-topic sequence numbers are
   monotonic. Cross-topic sequence numbers are monotonic only within
   a single Bus instance. Multi-bus deployments that need a global
   order need a `MultiBus` that stamps a global sequence at fan-in
   (§3.2). Most consumers don't care; the replication ledger
   (`16-research-to-runtime.md`) and chain witnesses
   (`09-phase-2-implications.md` §1) do.
7. **Authentication of publishers.** A subscriber can trust the Bus
   delivered the Pulse it claims was published, but can they trust
   the `source` field? For in-process publishers, yes (compile-time
   boundary). For cross-process publishers (HTTP webhook, remote
   agent) the Bus needs a signed-publish primitive. Punt to
   `32-safety-sandbox-provenance.md`.

## 9. Why this proposal is low-risk

Three concrete reasons the refactor is safe to commit to:

1. **The runtime primitive already exists.** We are giving a name,
   trait, and doc page to behavior that is live in production today.
   The `BroadcastBus` implementation is about 150 lines of wrapping
   around `tokio::sync::broadcast` plus a `VecDeque` for replay.
2. **The migration is compile-checkable.** Every call site that
   publishes or subscribes goes through `Bus::publish` or
   `Bus::subscribe`. If we rip out the current ad-hoc enums, the
   compiler tells us every place that needs updating. There is no
   "spooky action at a distance" coupling that would produce runtime
   surprises.
3. **No data shape changes.** Pulse reuses `Kind` and `Body` from
   the existing Engram taxonomy. No new serialization formats, no
   new on-disk representations, no migration scripts. The Substrate
   remains the only persistence surface.

Compare to the `roko-conductor → roko-learn` fix that doc 23
originally proposed (extract a `HealthMetrics` trait): that fix adds
a third trait surface and splits the failure-rate EMA logic across
two crates with a shared vocabulary. The Bus-first fix *removes* the
direct dependency without introducing a new trait — the shared
vocabulary becomes a topic string.

See `08-code-sketches.md` for the actual Rust signatures and
`06-refactoring-plan.md` for the migration steps. See
`33-observability-telemetry.md` for the metrics the Bus should
expose out of the box.

--- END 03-bus-as-first-class.md ---

# Batch REF03 — Promote Bus to kernel fabric across architecture + subsystem docs

**Refinement source**: `tmp/refinements/03-bus-as-first-class.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- `docs/00-architecture/07-substrate-trait.md` — rewrite or extend to introduce Bus as the transport fabric sibling to Substrate.
- Optionally add `docs/00-architecture/07b-bus-transport-fabric.md` as a full companion chapter.
- `docs/00-architecture/12-five-layer-taxonomy.md` — add Bus at L0 alongside Substrate.
- `docs/00-architecture/24-cross-section-integration-map.md` — reframe the EngineEventBus proposal as the Bus trait (now landed / planned).
- `docs/00-architecture/INDEX.md` — add Bus to the two-fabric summary.

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/03-bus-as-first-class.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.
- Bus trait signature documented (publish, subscribe, replay_since, current_seq, ring semantics).
- Topic + TopicFilter documented.
- doc-23 layer violation (roko-conductor → roko-learn) explicitly noted as dissolved by Bus topics.

## Required vocabulary (verify)

The verify step greps for: `Bus trait|Bus fabric|Bus primitive|Bus kernel|kernel Bus`

At least one changed file must mention it (case-insensitive) before the
batch is considered complete.

## Cross-references

Dependents — downstream refinements assuming this one has propagated
into docs: REF04, REF05, REF09, REF10, REF17, REF20, REF22, REF24, REF26, REF27

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
- Commit ready with message `refinements(REF03): Promote Bus to kernel fabric across architecture + subsystem docs`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
