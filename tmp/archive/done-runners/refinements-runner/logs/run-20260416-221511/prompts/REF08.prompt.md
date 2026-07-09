# Refinements Batch REF08

Run id: run-20260416-221511
Attempt: 1
Model: gpt-5.4
Reasoning: high
Refinement source: /Users/will/dev/nunchi/roko/roko/tmp/refinements/08-code-sketches.md
Target docs (candidates): docs/00-architecture/06-synapse-traits.md docs/00-architecture/07-substrate-trait.md docs/00-architecture/08-scorer-gate-router-composer-policy.md

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

Suggested parallel split for batch `REF08`:

- worker: add small Rust snippet sections to trait chapters (06, 07, 08)
  referencing the code sketches; no new chapter needed, snippets in-place.

## Canonical refinement source

This is the verbatim refinement proposal that this batch must propagate
into `docs/`. Treat it as the authoritative source. Do not edit this
file; only edit docs under `docs/`.

--- BEGIN 08-code-sketches.md ---

# Code Sketches

> **TL;DR**: Concrete Rust for the new `Bus` trait, the `Pulse` type,
> the `Datum` enum, the conversion methods, and a worked
> before/after migration of `roko-conductor` (the one with the doc-23
> layer violation).

> **For first-time readers**: This is the "show me the Rust" doc. The
> types shown here target `roko-core` (Phase B of the refactoring plan
> in 06). If you want the why, read 01–05 first. If you want the when,
> read 06 last.

## 1. `crates/roko-core/src/pulse.rs` (new)

```rust
//! Pulse — the ephemeral medium of Roko's transport fabric.
//!
//! A Pulse is an in-flight event traveling on a [`Bus`](crate::Bus). Pulses
//! are typed, sequence-numbered, and timestamped. Unlike [`Engram`]s, they
//! are not content-addressed, not persisted, and not scored. They deliver
//! once and live briefly in the Bus ring buffer.
//!
//! Pulses may graduate to Engrams via [`Pulse::graduate`] when their
//! lineage becomes forensically relevant (gate verdicts, process exits,
//! safety events). Pulses that don't graduate (heartbeats, UI refreshes,
//! token-chunk stream samples) vanish when the ring wraps.
//!
//! See `docs/00-architecture/02b-pulse-ephemeral-event.md` for the
//! conversion law and graduation policy.

use crate::{Body, ContentHash, Engram, EngramBuilder, Kind, Provenance, Decay, Score};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// An in-flight event on a Bus.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pulse {
    /// Topic-local monotonic sequence number. Unique within a
    /// (bus-instance, topic) pair.
    pub seq: u64,

    /// The topic this Pulse was published to.
    pub topic: Topic,

    /// Semantic category. Reuses [`Kind`] from the Engram taxonomy so
    /// that a Pulse and the Engram it may graduate into carry the
    /// same kind.
    pub kind: Kind,

    /// Payload. Reuses [`Body`] for the same reason.
    pub body: Body,

    /// Unix milliseconds when the Pulse was published.
    pub emitted_at_ms: i64,

    /// Lightweight origin attribution.
    pub source: PulseSource,

    /// Optional ContentHash of an Engram that contextualizes this
    /// Pulse. E.g. an `agent.msg.chunk` Pulse references the Task
    /// Engram its chunk belongs to.
    pub lineage_hint: Option<ContentHash>,

    /// Optional distributed-trace id.
    pub trace_id: Option<TraceId>,
}

/// Origin attribution for a Pulse.
///
/// Heavier than nothing, lighter than Engram's full `Provenance`.
/// Upgraded to full Provenance at graduation time.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PulseSource {
    /// Component that published the Pulse ("roko-orchestrator",
    /// "roko-agent-server:claude-sonnet-4-6", …).
    pub component: String,
    /// Optional agent or session identifier.
    pub agent_id: Option<String>,
}

/// Trace id for distributed tracing (W3C traceparent-shaped).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceId(pub [u8; 16]);

impl Pulse {
    /// Graduate this Pulse into an Engram for Substrate storage.
    pub fn graduate(
        &self,
        provenance: Provenance,
        decay: Decay,
        score: Score,
        tags: BTreeMap<String, String>,
    ) -> Engram {
        let lineage = self.lineage_hint.clone().into_iter().collect();
        EngramBuilder::new(self.kind.clone(), self.body.clone())
            .created_at_ms(self.emitted_at_ms)
            .provenance(provenance)
            .decay(decay)
            .score(score)
            .lineage(lineage)
            .tags(tags)
            .build()
    }
}

/// A topic string. Canonical form is dot-separated lowercase,
/// e.g. "gate.verdict.emitted".
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Topic(pub String);

impl Topic {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn matches(&self, filter: &TopicFilter) -> bool {
        filter.matches(self)
    }
}

/// Declarative filter for Bus subscriptions.
#[derive(Clone, Debug)]
pub enum TopicFilter {
    Exact(Topic),
    Glob(String),
    AnyOf(Vec<Topic>),
    All,
    And(Box<TopicFilter>, Box<TopicFilter>),
    Or(Box<TopicFilter>, Box<TopicFilter>),
    Not(Box<TopicFilter>),
}

impl TopicFilter {
    pub fn matches(&self, topic: &Topic) -> bool {
        match self {
            TopicFilter::Exact(t) => topic == t,
            TopicFilter::Glob(pattern) => glob_match(pattern, topic.as_str()),
            TopicFilter::AnyOf(ts) => ts.iter().any(|t| t == topic),
            TopicFilter::All => true,
            TopicFilter::And(a, b) => a.matches(topic) && b.matches(topic),
            TopicFilter::Or(a, b) => a.matches(topic) || b.matches(topic),
            TopicFilter::Not(inner) => !inner.matches(topic),
        }
    }
}

/// Dot-aware wildcard matcher.
///
/// Rules:
/// - `pattern` and `topic` are split by `.` into segments.
/// - A literal segment matches only itself (case-sensitive).
/// - A `*` segment matches exactly one segment, any value.
/// - A `**` segment matches zero or more segments and must be the last
///   segment or followed by a literal segment.
///
/// Examples:
///   `agent.*`           matches `agent.msg` but NOT `agent.msg.chunk`
///   `agent.**`          matches `agent.msg`, `agent.msg.chunk`, `agent`
///   `agent.*.chunk`     matches `agent.msg.chunk` but NOT `agent.msg`
///   `gate.verdict.*`    matches `gate.verdict.emitted`
///   `**.tripped`        matches `conductor.circuit.tripped`
fn glob_match(pattern: &str, topic: &str) -> bool {
    glob_segments(
        &pattern.split('.').collect::<Vec<_>>(),
        &topic.split('.').collect::<Vec<_>>(),
    )
}

fn glob_segments(pat: &[&str], top: &[&str]) -> bool {
    match (pat.first(), top.first()) {
        (None, None) => true,
        (None, Some(_)) => false,
        (Some(&"**"), _) => {
            // ** matches zero or more segments; try each split.
            if pat.len() == 1 {
                return true;
            }
            // With a literal (or *) after, scan the topic looking for a
            // suffix that matches the rest of the pattern.
            for i in 0..=top.len() {
                if glob_segments(&pat[1..], &top[i..]) {
                    return true;
                }
            }
            false
        }
        (Some(&"*"), Some(_)) => glob_segments(&pat[1..], &top[1..]),
        (Some(pp), Some(tt)) if pp == tt => glob_segments(&pat[1..], &top[1..]),
        _ => false,
    }
}

#[cfg(test)]
mod glob_tests {
    use super::*;

    #[test]
    fn exact_match() {
        assert!(glob_match("agent.msg.chunk", "agent.msg.chunk"));
        assert!(!glob_match("agent.msg.chunk", "agent.msg"));
        assert!(!glob_match("agent.msg.chunk", "agent.msg.chunk.extra"));
    }

    #[test]
    fn single_star_matches_one_segment() {
        assert!(glob_match("agent.*", "agent.msg"));
        assert!(!glob_match("agent.*", "agent.msg.chunk"));
        assert!(!glob_match("agent.*", "agent"));
    }

    #[test]
    fn double_star_matches_zero_or_more() {
        assert!(glob_match("agent.**", "agent"));
        assert!(glob_match("agent.**", "agent.msg"));
        assert!(glob_match("agent.**", "agent.msg.chunk"));
        assert!(!glob_match("agent.**", "user.msg"));
    }

    #[test]
    fn middle_star() {
        assert!(glob_match("agent.*.chunk", "agent.msg.chunk"));
        assert!(!glob_match("agent.*.chunk", "agent.msg.body"));
    }

    #[test]
    fn trailing_literal_after_double_star() {
        assert!(glob_match("**.tripped", "conductor.circuit.tripped"));
        assert!(glob_match("**.tripped", "tripped"));
        assert!(!glob_match("**.tripped", "conductor.circuit.reset"));
    }
}
```

## 2. `crates/roko-core/src/traits.rs` — adding `Bus`

```rust
// ─── Bus ───────────────────────────────────────────────────────────────────

use crate::{Pulse, Topic, TopicFilter, error::Result};
use async_trait::async_trait;
use tokio::sync::mpsc;

/// Transport fabric for [`Pulse`]s.
///
/// See crate docs for the two-medium / two-fabric model and
/// `docs/00-architecture/07b-bus-transport-fabric.md` for the full spec.
#[async_trait]
pub trait Bus: Send + Sync {
    /// Publish a Pulse. Returns its global sequence number.
    async fn publish(&self, pulse: Pulse) -> Result<u64>;

    /// Subscribe to Pulses matching `filter`. Returns a receiver.
    async fn subscribe(&self, filter: TopicFilter) -> Result<BusReceiver>;

    /// Replay Pulses newer than `since_seq` matching `filter`, up to
    /// the ring's retention window.
    async fn replay_since(
        &self,
        since_seq: u64,
        filter: &TopicFilter,
    ) -> Result<Vec<Pulse>>;

    /// Current global sequence number.
    async fn current_seq(&self) -> Result<u64>;

    /// Total Pulses published.
    async fn total_published(&self) -> Result<u64>;

    /// Current ring buffer occupancy.
    async fn ring_len(&self) -> Result<usize>;

    /// Ring buffer capacity.
    fn ring_capacity(&self) -> usize;

    fn name(&self) -> &'static str {
        "unnamed_bus"
    }
}

/// Receiver handle for a Bus subscription.
pub struct BusReceiver {
    pub(crate) inner: mpsc::Receiver<Pulse>,
    pub(crate) last_seq: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl BusReceiver {
    pub async fn recv(&mut self) -> Option<Pulse> {
        let p = self.inner.recv().await?;
        self.last_seq
            .store(p.seq, std::sync::atomic::Ordering::Relaxed);
        Some(p)
    }

    pub fn last_seq(&self) -> u64 {
        self.last_seq.load(std::sync::atomic::Ordering::Relaxed)
    }
}
```

## 3. `crates/roko-core/src/datum.rs` (new)

```rust
//! `Datum` — the either-medium reference used by polymorphic operators.

use crate::{Body, ContentHash, Engram, Kind, Pulse};
use std::collections::BTreeMap;

/// A reference to either medium.
#[derive(Clone, Copy, Debug)]
pub enum Datum<'a> {
    Engram(&'a Engram),
    Pulse(&'a Pulse),
}

impl<'a> Datum<'a> {
    pub fn kind(&self) -> &'a Kind {
        match self {
            Datum::Engram(e) => &e.kind,
            Datum::Pulse(p) => &p.kind,
        }
    }

    pub fn body(&self) -> &'a Body {
        match self {
            Datum::Engram(e) => &e.body,
            Datum::Pulse(p) => &p.body,
        }
    }

    pub fn created_at_ms(&self) -> i64 {
        match self {
            Datum::Engram(e) => e.created_at_ms,
            Datum::Pulse(p) => p.emitted_at_ms,
        }
    }

    pub fn tags(&self) -> Option<&'a BTreeMap<String, String>> {
        match self {
            Datum::Engram(e) => Some(&e.tags),
            Datum::Pulse(_) => None,
        }
    }

    pub fn content_hash(&self) -> Option<&'a ContentHash> {
        match self {
            Datum::Engram(e) => Some(&e.id),
            Datum::Pulse(p) => p.lineage_hint.as_ref(),
        }
    }
}

impl<'a> From<&'a Engram> for Datum<'a> {
    fn from(e: &'a Engram) -> Self {
        Datum::Engram(e)
    }
}

impl<'a> From<&'a Pulse> for Datum<'a> {
    fn from(p: &'a Pulse) -> Self {
        Datum::Pulse(p)
    }
}
```

## 4. `crates/roko-std/src/bus/broadcast.rs` (new)

```rust
//! `BroadcastBus` — in-process Bus backed by `tokio::sync::broadcast`.

use async_trait::async_trait;
use roko_core::{Bus, BusReceiver, Pulse, Topic, TopicFilter, error::Result};
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use tokio::sync::{broadcast, mpsc, Mutex};
use std::collections::VecDeque;

pub struct BroadcastBus {
    tx: broadcast::Sender<Pulse>,
    seq: Arc<AtomicU64>,
    ring: Arc<Mutex<VecDeque<Pulse>>>,
    capacity: usize,
}

impl BroadcastBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity.max(16));
        Self {
            tx,
            seq: Arc::new(AtomicU64::new(0)),
            ring: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
            capacity,
        }
    }
}

#[async_trait]
impl Bus for BroadcastBus {
    async fn publish(&self, mut pulse: Pulse) -> Result<u64> {
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);
        pulse.seq = seq;
        {
            let mut ring = self.ring.lock().await;
            if ring.len() == self.capacity {
                ring.pop_front();
            }
            ring.push_back(pulse.clone());
        }
        // Lagging subscribers simply miss Pulses; that's the broadcast contract.
        let _ = self.tx.send(pulse);
        Ok(seq)
    }

    async fn subscribe(&self, filter: TopicFilter) -> Result<BusReceiver> {
        let mut rx = self.tx.subscribe();
        let (tx, mpsc_rx) = mpsc::channel(self.capacity);
        let last_seq = Arc::new(AtomicU64::new(0));
        let last_seq_cloned = last_seq.clone();
        tokio::spawn(async move {
            while let Ok(p) = rx.recv().await {
                if filter.matches(&p.topic) && tx.send(p).await.is_err() {
                    break;
                }
            }
        });
        Ok(BusReceiver::new(mpsc_rx, last_seq))
    }

    async fn replay_since(
        &self,
        since_seq: u64,
        filter: &TopicFilter,
    ) -> Result<Vec<Pulse>> {
        let ring = self.ring.lock().await;
        Ok(ring
            .iter()
            .filter(|p| p.seq > since_seq && filter.matches(&p.topic))
            .cloned()
            .collect())
    }

    async fn current_seq(&self) -> Result<u64> {
        Ok(self.seq.load(Ordering::SeqCst))
    }

    async fn total_published(&self) -> Result<u64> {
        Ok(self.seq.load(Ordering::SeqCst))
    }

    async fn ring_len(&self) -> Result<usize> {
        Ok(self.ring.lock().await.len())
    }

    fn ring_capacity(&self) -> usize {
        self.capacity
    }

    fn name(&self) -> &'static str {
        "broadcast_bus"
    }
}
```

## 5. Conductor port — before / after

### 5.1 Before (doc-23 layer violation)

```rust
// crates/roko-conductor/src/circuit.rs (simplified)
use roko_learn::gate_stats::GateFailureEma;   // <── L3 reaching into L2/cross-cut

pub struct CircuitBreaker {
    ema: GateFailureEma,
    threshold: f32,
}

impl CircuitBreaker {
    pub fn observe(&mut self, verdict: &Verdict) {
        self.ema.record(verdict.passed);
    }

    pub fn tripped(&self) -> bool {
        self.ema.rate() > self.threshold
    }
}
```

### 5.2 After (Bus-mediated, no cross-layer import)

```rust
// crates/roko-conductor/src/circuit.rs
use roko_core::{Bus, BusReceiver, Pulse, Topic, TopicFilter};

pub struct CircuitBreaker<B: Bus> {
    bus: std::sync::Arc<B>,
    threshold: f32,
    current_rate: std::sync::atomic::AtomicU32, // f32 bits
}

impl<B: Bus> CircuitBreaker<B> {
    pub async fn run(self: std::sync::Arc<Self>) {
        let mut rx = self
            .bus
            .subscribe(TopicFilter::Exact(Topic::new("gate.failure.rate")))
            .await
            .expect("bus subscribe");
        while let Some(pulse) = rx.recv().await {
            if let roko_core::Body::Json(v) = &pulse.body {
                if let Some(rate) = v.get("rate").and_then(|r| r.as_f64()) {
                    self.current_rate
                        .store((rate as f32).to_bits(), std::sync::atomic::Ordering::SeqCst);
                    if rate as f32 > self.threshold {
                        let _ = self
                            .bus
                            .publish(Pulse {
                                seq: 0, // filled by bus
                                topic: Topic::new("conductor.circuit.tripped"),
                                kind: roko_core::Kind::Custom("circuit.tripped".into()),
                                body: roko_core::Body::Json(serde_json::json!({
                                    "rate": rate,
                                    "threshold": self.threshold,
                                })),
                                emitted_at_ms: now_ms(),
                                source: roko_core::PulseSource {
                                    component: "roko-conductor".into(),
                                    agent_id: None,
                                },
                                lineage_hint: None,
                                trace_id: None,
                            })
                            .await;
                    }
                }
            }
        }
    }
}
```

And on the publisher side, `roko-learn` gains a policy that publishes
the rate:

```rust
// crates/roko-learn/src/policies/failure_rate.rs (new)
use roko_core::{Bus, Pulse, Topic};

pub struct FailureRatePolicy<B: Bus> {
    bus: std::sync::Arc<B>,
    window_ms: i64,
    samples: parking_lot::Mutex<Vec<(i64, bool)>>, // (ts, passed)
}

impl<B: Bus> FailureRatePolicy<B> {
    pub async fn run(self: std::sync::Arc<Self>) {
        let mut rx = self
            .bus
            .subscribe(roko_core::TopicFilter::Exact(Topic::new("gate.verdict.emitted")))
            .await
            .expect("bus subscribe");
        while let Some(pulse) = rx.recv().await {
            let passed = verdict_passed(&pulse);
            let now = pulse.emitted_at_ms;
            let rate = {
                let mut s = self.samples.lock();
                s.push((now, passed));
                s.retain(|(t, _)| now - t < self.window_ms);
                let fails = s.iter().filter(|(_, p)| !p).count() as f32;
                fails / (s.len() as f32).max(1.0)
            };
            let _ = self
                .bus
                .publish(Pulse {
                    seq: 0,
                    topic: Topic::new("gate.failure.rate"),
                    kind: roko_core::Kind::Metric,
                    body: roko_core::Body::Json(serde_json::json!({ "rate": rate })),
                    emitted_at_ms: now,
                    source: roko_core::PulseSource {
                        component: "roko-learn".into(),
                        agent_id: None,
                    },
                    lineage_hint: pulse.lineage_hint,
                    trace_id: pulse.trace_id,
                })
                .await;
        }
    }
}

fn verdict_passed(_pulse: &Pulse) -> bool {
    // extract from pulse.body json
    todo!()
}
```

`roko-conductor`'s `Cargo.toml` loses its `roko-learn` dependency.
Both crates now depend only on `roko-core`. Layer violation dissolved.

## 6. `PlanRevisionPolicy` — the self-hosting closure

```rust
// crates/roko-cli/src/plan_revision_policy.rs (new)
use roko_core::{Bus, Pulse, Topic, TopicFilter};

pub struct PlanRevisionPolicy<B: Bus> {
    bus: std::sync::Arc<B>,
    threshold: usize, // N consecutive failures
    failures: parking_lot::Mutex<std::collections::HashMap<String, usize>>,
}

impl<B: Bus> PlanRevisionPolicy<B> {
    pub async fn run(self: std::sync::Arc<Self>) {
        let mut rx = self
            .bus
            .subscribe(TopicFilter::Exact(Topic::new("gate.verdict.emitted")))
            .await
            .unwrap();
        while let Some(pulse) = rx.recv().await {
            let task_hash = task_hash_from(&pulse);
            let passed = verdict_passed(&pulse);
            let count = {
                let mut f = self.failures.lock();
                if passed {
                    f.remove(&task_hash);
                    continue;
                }
                let e = f.entry(task_hash.clone()).or_insert(0);
                *e += 1;
                *e
            };
            if count >= self.threshold {
                let _ = self
                    .bus
                    .publish(Pulse {
                        seq: 0,
                        topic: Topic::new("plan.revision.requested"),
                        kind: roko_core::Kind::Custom("plan.revision".into()),
                        body: roko_core::Body::Json(serde_json::json!({
                            "task_hash": task_hash,
                            "failure_count": count,
                            "last_verdict_pulse_seq": pulse.seq,
                        })),
                        emitted_at_ms: now_ms(),
                        source: roko_core::PulseSource {
                            component: "roko-cli:plan-revision".into(),
                            agent_id: None,
                        },
                        lineage_hint: pulse.lineage_hint.clone(),
                        trace_id: pulse.trace_id.clone(),
                    })
                    .await;
                self.failures.lock().remove(&task_hash);
            }
        }
    }
}
```

And the orchestrator subscribes to `plan.revision.requested` and
invokes `roko prd plan <slug>` with the failure context injected.
That's CLAUDE.md item 11 done in ~80 lines of policy code.

## 7. Test sketch — `Pulse::graduate` round trip

```rust
// crates/roko-core/tests/pulse_graduation.rs
use roko_core::{Pulse, Topic, Kind, Body, PulseSource, Provenance, Decay, Score};
use std::collections::BTreeMap;

#[test]
fn graduated_engram_has_stable_content_hash() {
    let p = Pulse {
        seq: 42,
        topic: Topic::new("gate.verdict.emitted"),
        kind: Kind::GateVerdict,
        body: Body::Json(serde_json::json!({"passed": true, "gate": "compile"})),
        emitted_at_ms: 1_700_000_000_000,
        source: PulseSource { component: "test".into(), agent_id: None },
        lineage_hint: None,
        trace_id: None,
    };
    let prov = Provenance::test_default("agent-a");
    let e1 = p.graduate(prov.clone(), Decay::None, Score::default(), BTreeMap::new());
    let e2 = p.graduate(prov, Decay::None, Score::default(), BTreeMap::new());
    assert_eq!(e1.id, e2.id, "graduation must be deterministic");
}

#[test]
fn graduation_preserves_kind_and_body() {
    // ...
}

#[test]
fn pulse_to_engram_to_pulse_roundtrip_preserves_kind() {
    // project Engram → Pulse, graduate → Engram, check kind/body equal
}
```

## 8. `PrdPublishPolicy` — the automatic plan generation closure

`CLAUDE.md` item 10 (trigger `prd plan` automatically when a PRD is
published) is the twin of §6's `PlanRevisionPolicy`. Same pattern:

```rust
// crates/roko-cli/src/prd_publish_policy.rs (new)
use roko_core::{Bus, Pulse, Topic, TopicFilter};

pub struct PrdPublishPolicy<B: Bus> {
    pub bus: std::sync::Arc<B>,
}

impl<B: Bus> PrdPublishPolicy<B> {
    pub async fn run(self: std::sync::Arc<Self>) {
        let mut rx = self
            .bus
            .subscribe(TopicFilter::Exact(Topic::new("prd.published")))
            .await
            .unwrap();

        while let Some(pulse) = rx.recv().await {
            let slug = match pulse.body.as_json().and_then(|v| v.get("slug")).and_then(|s| s.as_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            let _ = self
                .bus
                .publish(Pulse {
                    seq: 0,
                    topic: Topic::new("plan.generation.requested"),
                    kind: roko_core::Kind::Custom("plan.generation".into()),
                    body: roko_core::Body::Json(serde_json::json!({
                        "slug": slug,
                        "source_pulse_seq": pulse.seq,
                    })),
                    emitted_at_ms: now_ms(),
                    source: roko_core::PulseSource {
                        component: "roko-cli:prd-publish".into(),
                        agent_id: None,
                    },
                    lineage_hint: pulse.lineage_hint,
                    trace_id: pulse.trace_id,
                })
                .await;
        }
    }
}
```

The orchestrator subscribes to `plan.generation.requested` and invokes
`roko prd plan <slug>` — closing CLAUDE.md item 10 in ~40 lines.

## 9. Additional tests to land in Phase B

Beyond §7's graduation round-trip, the Phase-B kernel tests should
cover:

```rust
// tests/topic_filter_boolean.rs
#[test]
fn and_or_not_combinations() {
    let agent_msgs = TopicFilter::Glob("agent.msg.*".into());
    let not_chunk = TopicFilter::Not(Box::new(TopicFilter::Exact(
        Topic::new("agent.msg.chunk"),
    )));
    let combined = TopicFilter::And(Box::new(agent_msgs), Box::new(not_chunk));
    assert!(combined.matches(&Topic::new("agent.msg.started")));
    assert!(!combined.matches(&Topic::new("agent.msg.chunk")));
}

// tests/broadcast_ring_wrap.rs
#[tokio::test]
async fn slow_subscriber_misses_pulses_after_ring_wrap() {
    let bus = BroadcastBus::new(4);
    let mut rx = bus.subscribe(TopicFilter::All).await.unwrap();
    for i in 0..6 {
        bus.publish(test_pulse(i)).await.unwrap();
    }
    let mut received = Vec::new();
    while let Ok(Some(p)) = tokio::time::timeout(
        std::time::Duration::from_millis(10),
        rx.recv(),
    ).await {
        received.push(p.seq);
    }
    // Broadcast is lossy; exact count depends on tokio scheduling.
    // Assert invariant: if any received, they are contiguous from the tail.
    if !received.is_empty() {
        for w in received.windows(2) {
            assert_eq!(w[1], w[0] + 1, "gap in broadcast delivery");
        }
    }
}

// tests/replay_since.rs
#[tokio::test]
async fn replay_returns_everything_newer_than_cursor() {
    let bus = BroadcastBus::new(100);
    for i in 0..50 {
        bus.publish(test_pulse_with_topic(i, if i % 2 == 0 { "a" } else { "b" }))
            .await.unwrap();
    }
    let got = bus
        .replay_since(20, &TopicFilter::Exact(Topic::new("a")))
        .await
        .unwrap();
    let expected: Vec<u64> = (21..50).filter(|i| i % 2 == 0).collect();
    let actual: Vec<u64> = got.iter().map(|p| p.seq).collect();
    assert_eq!(actual, expected);
}

// tests/lineage_hint_preserved.rs
#[tokio::test]
async fn graduated_engram_retains_lineage_from_pulse_hint() {
    let parent_hash = test_engram_hash("parent");
    let p = test_pulse_with_lineage(parent_hash.clone());
    let e = p.graduate(
        Provenance::test_default("test"),
        Decay::None,
        Score::default(),
        BTreeMap::new(),
    );
    assert_eq!(e.lineage.len(), 1);
    assert_eq!(e.lineage[0], parent_hash);
}
```

## 10. Engram → Pulse projection

The reverse direction (§3 of `02-engram-vs-pulse.md`) is a short impl:

```rust
impl Engram {
    /// Project this Engram as a Pulse for broadcast to live subscribers.
    /// Used e.g. by Substrate impls to publish `substrate.engram.stored`
    /// right after a successful put.
    pub fn to_pulse(
        &self,
        topic: Topic,
        seq: u64,
        source: PulseSource,
    ) -> Pulse {
        Pulse {
            seq,
            topic,
            kind: self.kind.clone(),
            body: self.body.clone(),
            emitted_at_ms: self.created_at_ms,
            source,
            lineage_hint: Some(self.id.clone()),
            trace_id: None,
        }
    }
}
```

The Substrate's standard bridge — "emit a Pulse for every successful
put" — wires this into the put method:

```rust
async fn put(&self, engram: Engram) -> Result<ContentHash> {
    let hash = engram.id.clone();
    // ... actual persistence work ...
    if let Some(bus) = &self.bus {
        let p = engram.to_pulse(
            Topic::new("substrate.engram.stored"),
            0,
            PulseSource {
                component: "substrate:file".into(),
                agent_id: None,
            },
        );
        let _ = bus.publish(p).await;   // best-effort — persistence already succeeded
    }
    Ok(hash)
}
```

The substrate-engram-stored topic is the bridge that lets Policy
decide over live Bus Pulses even when the event was produced by a
Substrate put (see `04-operators-generalized.md` §8).

All code here is sketch-level; the actual Phase-B implementation will
need `EngramBuilder` helpers, `Provenance::from_pulse_source`, and
Tokio test setup. But the shape is correct and composable from what's
already in `roko-core` and `roko-runtime`.

--- END 08-code-sketches.md ---

# Batch REF08 — Code sketches appendix + inline snippets in trait chapters

**Refinement source**: `tmp/refinements/08-code-sketches.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- `docs/00-architecture/06-synapse-traits.md` — short Rust snippets illustrating the Bus trait, Pulse struct, Datum enum.
- `docs/00-architecture/07-substrate-trait.md` — minimal query_similar signature snippet.
- `docs/00-architecture/08-scorer-gate-router-composer-policy.md` — updated signatures for each operator (matches REF04's rewrite; confirm consistency).

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/08-code-sketches.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.
- Snippets are illustrative, not normative. Cross-link to `tmp/refinements/08-code-sketches.md` for the full sketch.

## Required vocabulary (verify)

The verify step greps for: ````rust|pub trait|pub struct`

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
- Commit ready with message `refinements(REF08): Code sketches appendix + inline snippets in trait chapters`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
