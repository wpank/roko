# Refinements Batch REF01

Run id: run-20260416-221511
Attempt: 1
Model: gpt-5.4
Reasoning: high
Refinement source: /Users/will/dev/nunchi/roko/roko/tmp/refinements/01-critique-one-noun.md
Target docs (candidates): docs/00-architecture/INDEX.md docs/00-architecture/06-synapse-traits.md docs/00-architecture/23-architectural-analysis-improvements.md docs/INDEX.md

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

Suggested parallel split for batch `REF01`:

- worker: rewrite `docs/00-architecture/06-synapse-traits.md` to soften the
  "one noun, six verbs" claim and point forward to the two-medium / two-fabric
  refinement.
- worker: update `docs/00-architecture/INDEX.md` and `docs/INDEX.md` to lead
  with the new framing.
- worker: annotate `docs/00-architecture/23-architectural-analysis-improvements.md`
  with a footer noting which audit items the refactor dissolves.

## Canonical refinement source

This is the verbatim refinement proposal that this batch must propagate
into `docs/`. Treat it as the authoritative source. Do not edit this
file; only edit docs under `docs/`.

--- BEGIN 01-critique-one-noun.md ---

# Critique: "One Noun, Six Verbs" Is Selling Roko Short

> **TL;DR**: The current foundational framing conflates two distinct data
> shapes, pretends the event bus isn't an architectural primitive, and
> stretches trait signatures to cover cases they don't naturally fit. The
> framing should evolve, not just have "Signal" renamed to "Engram."

> **For first-time readers**: Roko's current docs describe the system as
> "one noun (Engram — a durable, hashed, scored record) and six verb traits
> (Substrate, Scorer, Gate, Router, Composer, Policy)." This doc argues that
> framing no longer matches the code. The subsequent docs (02–09) propose the
> replacement — a two-medium, two-fabric, six-operator kernel. Read this one
> first for the diagnosis; read 02 and 03 for the cure.

## 1. Where the phrase comes from

The phrase appears verbatim in at least three places:

- `crates/roko-core/src/lib.rs` lines 5–15: "The entire Roko system is built
  from **one noun** ([`Engram`]) and **six verbs**".
- `docs/00-architecture/INDEX.md` abstract: "The Synapse Architecture is
  Roko's compositional foundation: one noun (Engram) and six verb traits".
- `docs/00-architecture/06-synapse-traits.md` §1.1: "The number six is not
  arbitrary. It emerged from analyzing the complete Roko design corpus…
  Every capability, without exception, reduces to one of these six operations."

`CLAUDE.md:59` also repeats it ("1 noun (Signal) + 6 verb traits…") with
the old name still attached — which is itself evidence that the framing is
stale.

## 2. Problem A — There are already two data shapes

### 2.1 Engram (durable)

`crates/roko-core/src/engram.rs` defines the content-addressed,
lineage-bearing record:

```
id: ContentHash           // BLAKE3(kind + body + author + tags)
kind: Kind
body: Body
tags: BTreeMap<String, String>
created_at_ms: i64
decay: Decay              // None | HalfLife | Ttl | Ebbinghaus
score: Score              // 7-axis appraisal
lineage: Vec<ContentHash> // parent Engrams (audit DAG)
provenance: Provenance
attestation: Option<Attestation>
```

This is an **artifact**. It has an identity, a DAG position, a decay curve,
a trust chain, and eventually a cryptographic attestation. It is designed to
survive, to be audited, and to be promoted on-chain.

### 2.2 Envelope / event (ephemeral)

`crates/roko-runtime/src/event_bus.rs:59` defines a completely separate
shape:

```
pub struct Envelope<E> {
    pub seq: u64,
    pub emitted_at_ms: i64,
    pub event: E,   // user-provided generic event type
}
```

This is an **in-flight message**. It has a sequence number and a timestamp
but no hash, no lineage, no decay, no provenance, no score. It is designed
to fan out to subscribers and live briefly in a ring buffer for replay.

The EventBus is parameterized over the user's event type `E`. Every caller
invents its own event enum — `OrchestrationEvent`, `AgentEvent`, `UiEvent`,
and so on. None of them are Engrams. None of them need to be.

### 2.3 The architecture cannot explain this cleanly today

Because the docs say "one noun, Engram," every subsystem that wants to
communicate either (a) forces its messages through the EventBus and calls
them "events" (undocumented in architecture docs, invisible to the
universal loop), or (b) materializes everything as Engrams and writes to
the Substrate, which is expensive and wrong for things like heartbeat
ticks or cancellation signals.

The docs compensate with phrases like "Policy observes a stream of
signals," but never acknowledge that those streams are Envelopes on a
Bus, not Engrams in a Substrate.

## 3. Problem B — The bus isn't part of the architectural lexicon

`docs/00-architecture/24-cross-section-integration-map.md` §6 openly
proposes an `EngineEventBus` to fix cross-section integration, with this
diagnosis:

> Currently subsystems communicate through direct compile-time dependencies,
> leading to the `roko-conductor → roko-learn` layer violation flagged in
> doc 23. An event bus inverts the dependency: subsystems publish and
> subscribe to typed topics, and the bus is the integration map.

That proposal is 100% correct, and the bus already exists in
`roko-runtime`. What's missing is architectural recognition: the Bus is
not called out in the five-layer taxonomy, is not listed among the six
traits, and has no `docs/00-architecture/XX-bus.md` sibling to the
Substrate deep-dive at `docs/00-architecture/07-substrate-trait.md`.

This is why `roko-conductor` reaches across layers into `roko-learn`. If
the Bus were a kernel primitive, the conductor would subscribe to a
`gate.verdict` topic and `roko-learn` would be just another subscriber
computing EMAs — and the dependency would flow the right way.

## 4. Problem C — The trait signatures stretch to fit

The trait definitions in `crates/roko-core/src/traits.rs` claim to operate
uniformly on `&Engram`:

- `Substrate::put(signal: Engram)` — fine, Engram is what gets stored.
- `Scorer::score(signal: &Engram, ctx: &Context) -> Score` — fine when
  scoring a stored Engram, awkward when scoring a live event (you have to
  materialize).
- `Gate::verify(signal: &Engram, ctx: &Context) -> Verdict` — fine, gates
  verify Engrams.
- `Router::select(candidates: &[Engram], ctx: &Context) -> Option<Selection>`
  — mostly fine, but model routing and tool selection don't really produce
  Engram candidates; they produce *choices*, which are then logged as
  Engrams after the fact.
- `Composer::compose(signals: &[Engram], budget, scorer, ctx) -> Result<Engram>`
  — fine, the Composer's output is an Engram (the prompt).
- `Policy::decide(stream: &[Engram], ctx: &Context) -> Vec<Engram>` — this
  one is the tell. "Stream of Engrams" is a workaround for "stream of
  Pulses." Conductor watchers, circuit breakers, and heartbeat policies
  all want to react to live events, not to historical Engrams. Today they
  either (a) convert Envelopes to synthetic Engrams, or (b) bypass the
  trait entirely and subscribe to the EventBus directly.

`docs/00-architecture/23-architectural-analysis-improvements.md` §2.2
explicitly acknowledges this:

> Telemetry emission (metrics, traces) — current implementation:
> `Policy::decide(&[], ctx)` returning metric Engrams. Fit quality:
> Adequate. Empty stream input is awkward but functional.

"Adequate" and "awkward but functional" are the usual symptoms of an
abstraction doing two jobs.

## 5. Problem D — The universal loop hides three different sense sources

`docs/00-architecture/09-universal-cognitive-loop.md` step 1 is "PERCEIVE
→ Substrate.query() → What is happening?" In practice the agent runtime
perceives three different ways:

1. **Substrate.query** — durable Engrams. Used for context retrieval,
   episode lookup, plan discovery.
2. **Bus.subscribe** — live Pulses. Used for process lifecycle, approval
   requests, cancellation, circuit-breaker trips, gate verdicts in
   flight, token streams.
3. **External I/O** — WebSocket chunks from the LLM, stdout from a tool
   subprocess, HTTP requests on `roko-serve`. These are the edge of the
   runtime; they produce Pulses (and eventually Engrams), but they aren't
   either of the above.

Flattening these three into "Substrate.query" either forces everything
through hash-and-store (expensive, wrong for heartbeats) or quietly
routes most real work outside the loop description (the status quo).

## 6. Problem E — The name "Signal" is sitting idle

The rename `Signal → Engram` in code (per
`docs/00-architecture/01-naming-and-glossary.md` and verified by
`grep -rn`: Engram 877, Signal 5) freed an excellent name. "Signal" in
engineering usage *means* an in-flight event — a notification, an
interrupt, a pub/sub message. It is the natural name for what
`EventBus<E>::Envelope<E>` carries.

Leaving the name idle while carrying a stale "Signal = Engram" disclaimer
in every doc is a missed opportunity.

## 7. What the critique does *not* say

A few things that are correct and should not change:

- The **six traits themselves** are the right decomposition. Doc 23's
  audit of all 131 trait implementations showed no 7th trait is needed.
  The traits' *signatures* should generalize; the trait *set* is fine.
- The **five-layer taxonomy** is right. Strictly-downward dependencies
  is the correct rule. The Bus belongs at L0, same as Substrate.
- The **three-speed model** (Gamma / Theta / Delta) is right and
  orthogonal to any of this.
- The **three cross-cuts** (Neuro / Daimon / Dreams) are right as
  trait-object injections across layers.
- The **content-addressed DAG** is right and is the core innovation. The
  critique is that not every message needs to be a hashed DAG node.

## 8. Summary

The "one noun, six verbs" framing was a useful mnemonic at the start of
the project. It remains useful for explaining the durable half of the
system. But the runtime grew a second half — the bus and its messages —
that the framing doesn't acknowledge, and the gap now shows up as layer
violations, awkward trait usage, leaky loop descriptions, and a stale
Signal-vs-Engram disclaimer in every foundational doc.

The rest of this folder proposes the minimal refactor that dissolves
these problems without throwing away anything that works today.

## 9. Smoke tests for the critique

If this critique is right, three predictions should hold when someone greps
the codebase today:

1. **Ad-hoc event enums exist in multiple crates.** Run
   `rg 'enum (Orchestration|Agent|Ui|Conductor)Event' crates/` — each hit
   is a subsystem that invented its own bus vocabulary because the kernel
   didn't have one.
2. **Polling loops appear where subscriptions should.** Look for `loop { sleep(..); query(..); }`
   patterns in `crates/roko-cli/src/tui/` — the confirmed P0 in
   `tmp/ux-followup/12-tui-event-parity.md` is the exemplar.
3. **`Policy::decide(&[], ctx)` appears with empty slices.** Run
   `rg 'decide\(\s*&\[\]' crates/` — every hit is a Policy that really
   wanted to subscribe to a stream but was forced to materialize nothing as
   a zero-length Engram slice.

If any of these three predictions fails, the critique should be tempered.
As of 2026-04-16, all three hold. See `08-code-sketches.md` for the
conductor-port worked example that makes the fix concrete.

## 10. What this critique is and isn't

This is **not** a critique of the authors who wrote "one noun, six verbs."
It was the right framing for the codebase at the time it was written. The
framing outgrew itself the moment the Bus landed in `roko-runtime` and the
TUI became a live consumer — roughly, the moment Roko stopped being a
batch "query → respond → store" loop and started being an agent runtime
with subprocesses, streams, and subscribers.

The refactor this folder proposes is the natural next chapter, not a repudiation.
Doc 06 (refactoring plan) stages it so that no subsystem breaks. Doc 21
discusses which pieces benefit from a clean rewrite versus an incremental
edit. If the rewrites in 21 are too aggressive, the incremental path in 06
still dissolves the three problems above. Either way, the critique stands.

--- END 01-critique-one-noun.md ---

# Batch REF01 — Critique "one-noun, six-verbs" across docs/00-architecture

**Refinement source**: `tmp/refinements/01-critique-one-noun.md` (injected
above under "Canonical refinement source"). That source is the diagnosis;
your job is to propagate it into the canonical `docs/` so the existing
documentation no longer advertises the reductive framing as the last
word.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies, but
the primary candidates are:

- `docs/00-architecture/INDEX.md` — the chapter index; lead paragraph
  still says "one noun, six verbs" in most corpora.
- `docs/00-architecture/06-synapse-traits.md` — "The number six is not
  arbitrary" passage; soften and forward-link to the
  two-medium / two-fabric refinement.
- `docs/00-architecture/23-architectural-analysis-improvements.md` —
  audit doc. Add a footer noting that §2.2 "Adequate / awkward but
  functional" trait fits and §3.2 roko-conductor → roko-learn violation
  are the motivating evidence for the two-fabric reframing.
- `docs/INDEX.md` — top-level index. Update any "one noun, six verbs"
  one-liners.

## Required outputs

- Every updated doc retains its filename and high-level structure.
- The lead paragraph / abstract of `docs/00-architecture/INDEX.md`
  (and `docs/INDEX.md` if it carries a similar framing) evolves to
  something like:
  > Roko's kernel has two mediums (durable Engram + ephemeral Pulse)
  > moving through two fabrics (Substrate + Bus), acted on by six
  > operators, across five layers at three speeds with three
  > cross-cuts.
- A cross-reference to `tmp/refinements/01-critique-one-noun.md` and
  the remainder of the refinements folder appears near the top of any
  file you touch.
- No line asserts "one noun, six verbs" as the complete story. If the
  original phrase is retained for historical context, it is framed as
  "the original mnemonic; see the two-medium / two-fabric refinement."
- `06-synapse-traits.md` adds a prominent "See also" section pointing at
  the refinements folder and notes the signature generalization
  (covered in REF04).

## Cross-references

Dependents (downstream refinements that assume this critique has been
acknowledged in docs): REF02, REF03, REF04, REF05, REF07.

## Rules

Follow all rules in `context-pack/00-REFINEMENTS-RULES.md`:

- Only touch files under `docs/`.
- Substantive edits — no placeholders.
- Any retired terms ("Signal = Engram" disclaimer, `Bardo`, `Golem`,
  `Mori`, `Grimoire`, `Styx`, `Clade`) must either be removed or
  explicitly framed as retired.
- Required new vocabulary for this batch (verify): words matching
  `two mediums|two fabrics|six operators` should appear in at least
  one of the changed files.

## Done when

- The diff gate, scope gate, terminology gate, and required-term gate
  all pass.
- A commit message `refinements(REF01): Critique ...` is ready for the
  runner.
- Final message lists: which files changed, which retired disclaimers
  were removed, and which cross-references were added.
