# Refinements Batch REF11

Run id: run-20260416-221511
Attempt: 1
Model: gpt-5.4
Reasoning: high
Refinement source: /Users/will/dev/nunchi/roko/roko/tmp/refinements/11-hyperdimensional-substrate.md
Target docs (candidates): docs/06-neuro/ docs/00-architecture/02-engram-data-type.md docs/00-architecture/07-substrate-trait.md docs/00-architecture/27-temporal-knowledge-topology.md

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

Suggested parallel split for batch `REF11`:

- worker: update `docs/06-neuro/` INDEX and relevant files with HDC-per-Engram
  framing and the default encoder sketch.
- worker: update `docs/00-architecture/02-engram-data-type.md` with the
  fingerprint field.
- worker: update `docs/00-architecture/07-substrate-trait.md` with the
  `query_similar` method.
- worker: update `docs/00-architecture/27-temporal-knowledge-topology.md` with
  HDC-cluster-driven tier progression.

## Canonical refinement source

This is the verbatim refinement proposal that this batch must propagate
into `docs/`. Treat it as the authoritative source. Do not edit this
file; only edit docs under `docs/`.

--- BEGIN 11-hyperdimensional-substrate.md ---

# Hyperdimensional Substrate

> **TL;DR**: Every Engram should carry a 10,240-bit HDC fingerprint as
> a first-class field, not as an optional side-table. Doing so turns
> similarity, consensus, stigmergy, analogy, and compositional memory
> into O(1) vector ops over a fabric that already exists. The `roko-primitives`
> crate is scaffolded for this; the Bus makes it the standard currency
> for *agreement* between agents.

> **For first-time readers**: Hyperdimensional Computing (HDC) represents
> concepts as very long (10,240-bit) binary vectors. Two random vectors of
> that length are nearly orthogonal; meaningful structure is composed by
> XOR (bind), majority-vote (bundle), and cyclic shift (permute). Cosine /
> Hamming similarity between two such vectors is ~1 ns with SIMD popcount.
> `roko-primitives` already has `HdcVector`; this doc proposes making it a
> field on every Engram (not just a side-table) and building the core
> operations around it.

## 1. Why HDC is the right choice for Roko

Hyperdimensional Computing (Kanerva 2009, "Hyperdimensional Computing:
An Introduction to Computing in Distributed Representation") uses very
high-dimensional binary or bipolar vectors (10,000+ bits) to represent
symbols, concepts, structures, and sequences in a uniform space. The
properties we need:

1. **Near-orthogonality**: two random HD vectors are nearly orthogonal.
   Noise tolerance is exponential in dimension.
2. **Compositionality**: bind (⊗, XOR for binary), bundle (+, majority
   vote), permute (ρ, cyclic shift) compose into structured meaning.
3. **Fast similarity**: cosine / Hamming over 10,240 bits is a cache
   line plus SIMD popcount — ≈1 ns per comparison.
4. **Holographic**: partial information still retrieves. Damage one
   fraction; the rest still works.
5. **Content-addressable**: similar inputs produce similar vectors
   without an index.

These properties are what the Substrate *pretends* to have today via
BLAKE3 content addressing (unique retrieval) and what it *needs* to
have to support collective intelligence (similarity-based consensus).
HDC is the bridge.

## 2. What `roko-primitives` already provides

`roko-primitives` has:
- `HdcVector` type (10,240 bits)
- Tier routing that uses it (partial wiring)
- Similarity computation

What it doesn't have:
- A first-class Engram field for the fingerprint
- A canonical encoder from Engram body → HDC vector
- HDC-based Substrate query primitives
- Integration with the Bus as a consensus channel

## 3. The missing field on Engram

Proposed addition to `roko-core::engram::Engram`:

```rust
pub struct Engram {
    // ... existing fields ...
    /// Hyperdimensional fingerprint of this Engram's content.
    /// Populated by the Substrate at `put()` time. None only if
    /// encoder is unavailable or disabled.
    pub fingerprint: Option<HdcVector>,
}
```

The fingerprint is computed deterministically from (kind, body) via an
encoder registered in `roko-primitives`. This makes it:

- Deterministic (same input → same fingerprint)
- Reproducible across nodes (deterministic encoder)
- Cheap to compute (microseconds)
- Cheap to compare (nanoseconds)

## 4. HDC-based Substrate queries

Today `Substrate::query` is filter-based: "give me all Engrams of Kind
X in tier Y from time T1 to T2". Add:

```rust
pub trait Substrate {
    // existing query(...) method stays

    /// Find Engrams whose fingerprint is within `radius` of `fp`.
    /// Returns up to `limit` results sorted by similarity descending.
    fn query_similar(
        &self,
        fp: &HdcVector,
        radius: f32,
        limit: usize,
    ) -> Vec<(EngramHash, f32)>;
}
```

This is *not* semantic search over an external embedding index. It's
native similarity over content we've already stored. Every Engram is
queryable by similarity the moment it lands.

### 4.1 Scale

At 10,240 bits per fingerprint, a single 1 GB RAM buffer holds about
800,000 fingerprints. Brute-force cosine (SIMD) comparison is
~10^9/second on a modern CPU. So `query_similar` against 800k Engrams is
< 1 ms. For larger scales, existing LSH techniques over HDC give
sub-ms retrieval at tens of millions.

This is the "competitive moat" at the tier: an agent runtime with
native, millisecond-latency similarity over its entire memory. Compare
to external vector stores (Pinecone, Weaviate): round-trip alone is
usually 10–100 ms, before query.

## 5. HDC as consensus currency

Consensus among agents today is a human or an orchestrator arbitrating
between outputs. With HDC:

### 5.1 Bundle-based consensus

Two agents produce outputs A and B on the same task. The Substrate
computes `fingerprint(A)` and `fingerprint(B)`. Consensus vector =
`bundle(fp_A, fp_B)`. If `similarity(fp_A, bundle) > threshold` and
`similarity(fp_B, bundle) > threshold`, the two agents *substantively
agree* (in the holographic sense) despite surface differences. This
catches "both said correct thing with different words".

### 5.2 Stigmergic pheromones as HDC vectors

Doc 09 §3 introduced stigmergy as Engrams in a shared Substrate. With
HDC, *pheromone strength along a direction* is represented as the HDC
vector's similarity to a "reward direction" vector. Deposits that point
in the same direction reinforce via bundling; deposits that point
elsewhere stay orthogonal. Natural, fast, and emergent.

This is the computational realization of ant colony optimization (Dorigo
1992) but with *semantic* pheromones rather than scalar counts.

### 5.3 Consensus Bus topic

`consensus.proposal.made` carries an HDC vector plus a claim. Each
subscribing agent publishes `consensus.vote.cast` with its own HDC
vector relative to the proposal. The orchestrator bundles the votes
and publishes `consensus.achieved` or `consensus.failed` based on
bundle similarity to the original proposal.

Three advantages over token-based voting:
- Disagreements about wording vs. substance become visible.
- Partial agreements can be detected ("agents agree on X, differ on Y").
- Swarm scales: bundling N vectors is still one vector.

## 6. Compositional memory via bind/bundle

The magic of HDC is that you can represent *structures* by composition:

```text
fp(turn_5) = bind(role_vector, agent_A) + bind(task_vector, T123) +
             bind(output_vector, output_hash) + bind(time_vector, t5)
```

Every Engram's fingerprint encodes its role, task, author, time, and
content in *one vector*. Queries can then decompose:

- "what was agent A doing at time t5?" → query_similar to
  `bind(agent_A, time_t5)`.
- "which Engrams relate to task T123?" → query_similar to
  `bind(task_vector, T123)`.

This replaces N indexes with one vector space. The Substrate becomes
holographic: every fingerprint carries its own context.

## 7. HDC as the decay mechanism

The existing decay model (`None`, `HalfLife`, `Ttl`, `Ebbinghaus`)
operates on Engram weights. HDC gives us a subtler decay: *vector noise
accumulation*. An Engram's effective fingerprint can be a weighted
blend of its original fingerprint and a noise vector, with the noise
weight growing over time. Old Engrams become *fuzzier* rather than
gone:

- A 1-year-old Engram matches broad categories but not specific ones.
- A 1-hour-old Engram matches both.

This is biologically faithful — human memory gets more categorical and
less episodic over time — and it's what enables *generalization* in a
Substrate. The Neuro tier-progression loop (Phase 4) uses exactly this
to promote specific episodes to semantic knowledge: as fingerprints
drift, similar ones cluster, and a cluster-center becomes a new
category Engram.

## 8. Anti-hallucination via HDC consistency

A hallucination is a claim whose fingerprint doesn't match the
fingerprints of its claimed supporting evidence. Concretely:

1. Agent produces output O claiming to be about X.
2. Substrate computes fp(O).
3. Substrate queries for Engrams with lineage tagged as supporting X.
   These have their own fingerprints.
4. If fp(O) is far from the bundle of supporting fingerprints, the
   output is *semantically disconnected* from its claimed support.
5. A `ConsistencyGate` subscribes to `agent.turn.completed`, runs this
   check, and publishes `gate.hallucination.detected` if threshold
   exceeded.

This isn't absolute hallucination detection — only semantic-drift
detection — but combined with provenance chains it's the highest-value
signal available without external truth. (The Gate pipeline then
cascades to expensive verifiers only on flagged cases, saving 90%+ of
verifier cost.)

## 9. Analogy and few-shot via HDC

Classical HDC result (Kanerva 1994, "The Binary Spatter Code"): analogy
solves via vector arithmetic. "Paris is to France as Tokyo is to ___"
becomes `fp(Tokyo) + (fp(France) - fp(Paris))`; query_similar returns
Japan. In Roko:

- Analogy-driven playbook retrieval: given a new task whose fingerprint
  is `fp(new)`, find the playbook whose fingerprint is
  `fp(playbook) ≈ fp(new) + (fp(old_task) - fp(old_playbook))`.
- Few-shot prompt construction: the Composer picks examples whose
  fingerprint-difference to the current input matches the successful
  pattern of a prior winning prompt.

These are not research projects; they're three-line queries against
the Substrate.

## 10. HDC as meta-state

Agents can carry an *identity fingerprint* — the bundle of all Engrams
authored by them over the last K turns. An agent's identity drifts as
they work. Observable properties:

- Two agents with similar identity fingerprints are doing similar work
  (automatic team discovery).
- An agent whose identity fingerprint changed sharply has changed
  domains (algedonic signal to orchestrator).
- Agent identity vectors can be *composed* via bind: `identity(A) +
  identity(B) - identity(C)` = "A and B, but not doing what C does".
  This is the algebraic foundation for team-building policies.

## 11. What to implement (concrete tasks)

### 11.1 Phase B.5 (between kernel landing and subsystem migration)

1. Add `fingerprint: Option<HdcVector>` to `Engram`. Default None.
2. Register a default encoder in `roko-primitives`: hash each bytestring
   word into HD space, bundle words with position-bind.
3. `FileSubstrate::put` populates the fingerprint at insert time if not
   already set. `FileSubstrate::query_similar` implemented via
   brute-force scan (fine for <1M Engrams).
4. `fingerprint` exposed on all HTTP/REST routes that return Engrams.
5. TUI F7 Substrate tab gains a "Similar to…" search box.

### 11.2 Phase C.5 (HDC consensus)

1. Bus topics: `consensus.proposal.made`, `consensus.vote.cast`,
   `consensus.achieved`, `consensus.failed`.
2. A `ConsensusPolicy` in `roko-learn` that accumulates votes and
   publishes outcomes.
3. `query_similar` used by Router when selecting among candidate
   Engrams (not only by score, but also by similarity to prior
   winners).

### 11.3 Phase D (HDC-native operations)

1. HDC-based `Kind::Playbook` retrieval: analogy-driven.
2. ConsistencyGate deployed as stream-gate in `roko-gate`.
3. HDC-powered Dreams consolidation: fingerprint clustering picks
   Engrams for promotion.

## 12. Why this is a competitive moat

Three converging facts:

1. No agent framework today has HDC as a core primitive. LangChain,
   LlamaIndex, CrewAI, AutoGen all rely on external vector stores for
   similarity. That's a 10–100 ms tax on every retrieval.
2. HDC has a 20-year research literature with concrete algorithms for
   binding, unbinding, cleanup, analogy, and sequential representation.
   None of this requires model training.
3. HDC is fundamentally compatible with the `Engram` concept —
   content-addressed, deterministic, compositional. Roko's data model
   was already HDC-shaped before anyone noticed.

An HDC-native Substrate is thus a moat Roko can build in weeks that
would take competitors months to replicate, because it requires
changing their core data model rather than bolting on a library.

## 13. Academic lineage

- **Kanerva 2009**: hyperdimensional computing foundational paper.
- **Plate 2003**: Holographic Reduced Representations — the real-valued
  analog.
- **Rachkovskij 2001**: binary spatter codes.
- **Levy & Gayler 2008**: vector-symbolic architectures survey.
- **Rahimi & Recht 2007**: random feature maps, theoretical tie to HD
  computing.
- **Olshausen & Field 1996**: sparse distributed representations in
  cortex — biological precedent.

This is a mature field, not speculative. The engineering path is
clear. Each citation becomes a Paper Engram once
`16-research-to-runtime.md` lands; the capacity and
near-orthogonality claims become testable hypotheses in Roko's own
replication ledger.

## 14. Canonical encoder — the default implementation

For Phase B.5 (per §11.1) the default Engram encoder needs to be
simple, deterministic, and fast. A rough sketch:

```rust
// roko-hdc/src/encoder.rs (new crate per 20-modularity-composability.md §2.2)
pub struct DefaultEncoder {
    word_memory: Arc<WordMemory>,   // hash -> HdcVector for reproducibility
    dim: usize,                     // 10,240 by default
}

impl DefaultEncoder {
    /// Encode an Engram's kind + body into a deterministic fingerprint.
    ///
    /// For textual bodies: tokenize, look up per-word vectors from
    /// word_memory (created on first sight, cached thereafter),
    /// permute by position, bundle. For structured bodies (JSON),
    /// bind each (key, value) pair and bundle the results.
    pub fn encode(&self, e: &Engram) -> HdcVector {
        let mut acc = HdcVector::zero(self.dim);
        acc = acc.bundle(&self.word_memory.for_kind(&e.kind));
        match &e.body {
            Body::Text(s) => acc = acc.bundle(&self.encode_text(s)),
            Body::Json(v) => acc = acc.bundle(&self.encode_json(v)),
            Body::Bytes(b) => acc = acc.bundle(&self.encode_bytes(b)),
        }
        for (k, v) in &e.tags {
            let kv = self.word_memory.for_key(k)
                .bind(&self.word_memory.for_value(v));
            acc = acc.bundle(&kv);
        }
        acc
    }

    fn encode_text(&self, text: &str) -> HdcVector {
        let mut acc = HdcVector::zero(self.dim);
        for (pos, word) in text.split_whitespace().enumerate() {
            let wv = self.word_memory.for_word(word);
            acc = acc.bundle(&wv.permute(pos as u32));
        }
        acc
    }

    fn encode_json(&self, v: &serde_json::Value) -> HdcVector {
        use serde_json::Value::*;
        match v {
            Object(o) => {
                let mut acc = HdcVector::zero(self.dim);
                for (k, vv) in o {
                    let kv = self.word_memory.for_key(k)
                        .bind(&self.encode_json(vv));
                    acc = acc.bundle(&kv);
                }
                acc
            }
            Array(a) => {
                let mut acc = HdcVector::zero(self.dim);
                for (i, vv) in a.iter().enumerate() {
                    acc = acc.bundle(&self.encode_json(vv).permute(i as u32));
                }
                acc
            }
            String(s) => self.encode_text(s),
            Number(n) => self.word_memory.for_number(n.as_f64().unwrap_or(0.0)),
            Bool(b) => self.word_memory.for_bool(*b),
            Null => HdcVector::zero(self.dim),
        }
    }

    fn encode_bytes(&self, b: &[u8]) -> HdcVector {
        // Hash bytes into a vector; for binary bodies this loses
        // semantic content but gives uniqueness. Specialized binary
        // encoders can be registered per-kind.
        HdcVector::from_hash(blake3::hash(b).as_bytes(), self.dim)
    }
}
```

`word_memory` is a cache — same word always produces the same vector
within a deployment. Cross-deployment determinism is achieved by
seeding `word_memory` from a BLAKE3 hash of the word (or key), so any
two deployments with the same seed produce the same vectors.

## 15. Encoder plurality

The default encoder above is generic. Specific Kinds benefit from
specialized encoders:

- `Kind::Plan` — encode tasks in order via position-permute, bind
  task IDs with dependency edges.
- `Kind::GateVerdict` — bind gate-name with pass/fail vector; bundle.
- `Kind::Transaction` (Phase 2+) — bind from/to addresses, amount,
  chain id.

Specialized encoders register via `roko_hdc::register_encoder::<K>(...)`.
The default catches everything else. This is the HDC analog of
trait-based dispatch: the kernel knows one function; domains
specialize it.

## 16. Cross-synergies with other refinements

HDC is one of the most load-bearing refinements because it multiplies
the value of everything else:

- **Demurrage (12)** uses HDC neighbor similarity to weight
  reinforcement: citing a rare Engram bumps balance more than citing a
  common one.
- **c-factor (13)** §2.2 — cognitive diversity is the pairwise
  distance between agents' HDC clouds. Without HDC, this metric has no
  efficient implementation.
- **Heuristics (14)** §4 — worldview clustering is community
  detection on heuristic fingerprints.
- **Research-to-runtime (16)** §2 — paper fingerprints let the
  Composer pull in the "most similar paper" for a situation without a
  separate embedding service.
- **Compose (beyond 21 §2.5)** — prompt construction via HDC cleanup
  picks templates whose fingerprint is closest to the current
  situation.
- **StateHub projection hashing (26)** — deduplicating identical
  deltas across consumers uses fingerprint-equality.

## 17. What can go wrong

HDC isn't magic. Three failure modes to watch:

1. **Encoder drift** — two encoders producing different vectors for
   the same input. Mitigation: encoder version in the fingerprint
   metadata; Substrate refuses to mix. A `fingerprint.encoder_version`
   field goes alongside the vector.
2. **Capacity exhaustion** — in theory 10,240 bits hold enormous
   structure, but bundling too many items crowds the space. The
   cleanup-to-codebook primitive helps, and per-Engram encoders
   shouldn't bundle more than ~1000 atomic items in a single vector.
   For bigger structures, compose a small set of sub-fingerprints
   lazily rather than one giant vector.
3. **Near-duplicates confusing retrieval** — if two Engrams are
   fingerprint-similar but semantically different (e.g. two
   error stacks that share keywords but are unrelated), retrieval
   returns both. Fix with tag-binding: `fp(stack) · fp(error_code)`
   separates them even when the text overlaps.

These aren't theoretical; they're what operators will encounter and
should be documented in the eventual `docs/00-architecture/XX-hdc.md`
chapter.

--- END 11-hyperdimensional-substrate.md ---

# Batch REF11 — HDC as first-class substrate field across neuro + architecture

**Refinement source**: `tmp/refinements/11-hyperdimensional-substrate.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- `docs/06-neuro/` — document HDC-per-Engram, encoder plurality, similarity/consensus/analogy.
- `docs/00-architecture/02-engram-data-type.md` — add fingerprint field.
- `docs/00-architecture/07-substrate-trait.md` — document query_similar.
- `docs/00-architecture/27-temporal-knowledge-topology.md` — HDC-cluster-driven tier progression.

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/11-hyperdimensional-substrate.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.


## Required vocabulary (verify)

The verify step greps for: `HDC|fingerprint`

At least one changed file must mention it (case-insensitive) before the
batch is considered complete.

## Cross-references

Dependents — downstream refinements assuming this one has propagated
into docs: REF12, REF13, REF14, REF19, REF20, REF31

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
- Commit ready with message `refinements(REF11): HDC as first-class substrate field across neuro + architecture`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
