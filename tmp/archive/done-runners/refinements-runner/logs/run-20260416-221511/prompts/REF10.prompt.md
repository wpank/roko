# Refinements Batch REF10

Run id: run-20260416-221511
Attempt: 1
Model: gpt-5.4
Reasoning: high
Refinement source: /Users/will/dev/nunchi/roko/roko/tmp/refinements/10-self-learning-cybernetic-loops.md
Target docs (candidates): docs/05-learning/ docs/00-architecture/11-dual-process-and-active-inference.md docs/00-architecture/16-autocatalytic-and-cybernetics.md docs/16-heartbeat/11-active-inference-state-space.md

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

Suggested parallel split for batch `REF10`:

- worker: add/update files under `docs/05-learning/` to describe the
  predict-publish-correct loop, per-operator calibration, CalibrationPolicy.
- worker: update `docs/00-architecture/11-dual-process-and-active-inference.md`
  with the FEP-as-literal implementation note.
- worker: update `docs/00-architecture/16-autocatalytic-and-cybernetics.md`
  with the Bus-as-feedback-nervous-system framing.
- worker: update `docs/16-heartbeat/11-active-inference-state-space.md` with
  prediction/outcome topic references.

## Canonical refinement source

This is the verbatim refinement proposal that this batch must propagate
into `docs/`. Treat it as the authoritative source. Do not edit this
file; only edit docs under `docs/`.

--- BEGIN 10-self-learning-cybernetic-loops.md ---

# Self-Learning & Cybernetic Feedback Loops

> **TL;DR**: Once the Bus exists as a first-class fabric, *every operator
> becomes a learner*, not just the three that already happen to be
> (CascadeRouter's bandit, EpisodePolicy's replay, experiments' A/B). The
> Free Energy Principle stops being a metaphor and becomes a concrete
> variational-inference loop: operators predict, publish predictions as
> Pulses, get corrected by later Pulses, and update. Roko becomes a
> self-modeling system whose prediction error is a first-class signal.

> **For first-time readers**: Roko already has three partial learners —
> `CascadeRouter` (model bandit), `EpisodeLogger` (turn replay), and
> `ExperimentStore` (prompt A/B). The six kernel operators (Scorer, Gate,
> Router, Composer, Policy + Substrate/Bus) were not designed as learners.
> This doc argues they *already are*, latent — and that the Pulse/Bus
> primitives from 02–03 make the predict-correct-update loop uniform across
> all of them. The payoff: active inference (Friston) becomes an
> implementation technique, not a metaphor. Read 02–05 for vocabulary; read
> 11 and 14 alongside this one for the HDC and heuristic substrates that
> make individual operator-level calibration stick.

## 1. The one thing the current design gets wrong about learning

Today, "learning" in Roko is three things stapled on the side:

1. **`CascadeRouter` bandit** — chooses a model per turn and updates from
   reward (`roko-learn/src/cascade_router.rs`).
2. **Episode replay** — stores full turns in `.roko/episodes.jsonl` and
   periodically distils them into playbooks (`roko-learn/src/episode.rs`).
3. **Prompt A/B experiments** — `ExperimentStore` picks among variants and
   tracks win-rate (`roko-learn/src/experiments.rs`).

Everything else — Scorer weights, Gate thresholds, Router topologies
beyond cascade, Composer template selection, Policy parameters — is
hardcoded or configured, never learned.

The pattern that should emerge naturally from the two-fabric refactor:
**every operator is a predictor; every downstream Pulse is a potential
training signal**. The Bus is the universal feedback channel.

## 2. Active inference as a literal implementation

### 2.1 The Free Energy Principle in one paragraph

Friston's FEP says an agent maintains a generative model of its world,
makes predictions that minimize *expected* free energy (prediction error
+ complexity penalty), acts to make its predictions true, and updates
the generative model when they fail. Most AI implementations treat it
as a metaphor. With Bus + Pulse, it can be literal.

### 2.2 The predict-publish-correct loop

For any operator `O` that produces output `y` from input `x`:

```text
1. O.predict(x) publishes Pulse{topic = "O.prediction", body = y_hat, lineage_hint = x.hash}
2. Downstream reality publishes Pulse{topic = "O.outcome", body = y_true, lineage_hint = x.hash}
3. A learning policy subscribes to both, joins by lineage_hint, and
   publishes Pulse{topic = "O.error", body = (y_hat, y_true, loss)}
4. O subscribes to its own error topic and updates internal state.
```

This is active inference in four bullets. It runs on top of the Bus
trait; no new primitive needed.

### 2.3 Concrete example: Scorer calibration

The Scorer assigns a 7-axis Score to every candidate. Today, Score axes
are tuned by hand. With active inference:

- Scorer publishes `scorer.prediction` Pulses containing
  `(engram_hash, predicted_reward)`.
- The GateVerdict stream publishes `gate.verdict.emitted` with
  `(engram_hash, success)`.
- A `ScorerCalibrationPolicy` subscribes to both, builds empirical
  calibration curves per-axis, and publishes `scorer.weights.updated`
  Pulses.
- Scorer subscribes to `scorer.weights.updated` and reloads weights.

Nothing else in the system changes. The Scorer became self-calibrating.
Every axis now has an empirical reliability curve. Humans can inspect
it in the TUI (F4 Learn tab already exists).

### 2.4 Per-operator calibration is the breakthrough

Most "learning" in agent systems is at two extremes: weights inside the
LLM (pre-training; we don't touch) and bandit over models (CascadeRouter;
we do, but only that). Per-operator calibration is the missing middle
tier. With the Bus, every operator gets one essentially for free.

| Operator | Predicts | Outcome signal | Update policy |
|---|---|---|---|
| Scorer | 7-axis reward | Gate verdict + episode reward | Online least-squares on each axis |
| Router | selected action will succeed | Gate verdict | Contextual bandit (already in cascade; generalize) |
| Composer | prompt will fit budget + win gate | Token count + gate verdict | Template EMA; template bandit |
| Gate | task will succeed post-patch | Next gate verdict + regression tests | Threshold EMA (already partial in `adaptive.rs`) |
| Policy | decision will improve metric | Metric Pulse after decision | Per-policy online calibration |
| Substrate | Engram tier is correct | Query frequency + recency | Tier-promotion Markov chain |

Six self-calibrating operators instead of three partially-adaptive
subsystems. Scale: every call in the system produces training data for
something.

## 3. Closed-loop prompt optimization (DSPy-style, but native)

Stanford's DSPy project (Khattab et al. 2023) compiles prompts rather
than writing them: you describe a program in modules, provide a metric,
and DSPy optimizes prompts by bootstrapping examples and A/B testing.
Roko's Composer plus the Bus gives us a tighter, native version:

### 3.1 The Composer as a compiler target

Composer templates become first-class `TemplateEngram`s (stored,
versioned, content-addressed). Each template has a vacant "slot" for
the input Engram and a `SuccessMetric` field linking to a Gate
pipeline whose verdict is the template's reward.

### 3.2 The optimization loop

```text
1. ExperimentPolicy publishes Pulse{topic = "template.variant.proposed",
   body = TemplateEngram'} containing a mutation of an existing template
   (rewrite an instruction, add an example, shorten a preamble).
2. The Composer, under a feature flag, routes N% of traffic to the new
   variant via an A/B split (exists today in ExperimentStore).
3. Gate verdicts land on the Bus; the ExperimentPolicy accumulates
   wins/losses by template hash.
4. After M trials, ExperimentPolicy publishes Pulse{topic = "template.promoted"}
   if the variant beats control by epsilon; otherwise "template.rejected".
5. The Dreams consolidation loop (Phase 5C) subscribes to "template.promoted"
   Pulses across agents and distils the winners into a meta-template
   manifesto that mutation policies draw from.
```

### 3.3 The meta-level: mutation policies are themselves learned

The mutation itself (rewrite, add example, shorten) is chosen by a
`MutationPolicy` that's itself learning what kinds of mutations tend to
work on what kinds of templates. The Bus topic `mutation.outcome`
carries (mutation_type, template_hash, won_ab). Over time, the
MutationPolicy becomes a prompt-evolution genetic algorithm whose
fitness function is the Gate.

### 3.4 Why this is better than DSPy

DSPy runs offline compilation passes on human-chosen metrics. Roko's
version runs *continuously* during normal operation. Every production
turn is a training sample. DSPy is batch; Roko is online.

## 4. Cybernetic feedback hierarchy

Stafford Beer's Viable System Model describes five recursive systems,
each a feedback loop over the one below. Roko's three speeds map
cleanly:

- **Gamma (5–15 s)**: Beer's System 1 — operations. Individual agent
  turns, token streams, immediate gate decisions.
- **Theta (~75 s)**: Beer's System 2+3 — coordination and internal
  regulation. The orchestrator's plan-level decisions, circuit breakers,
  efficiency policies.
- **Delta (hours)**: Beer's System 4+5 — intelligence and identity.
  Dreams consolidation, Neuro tier progression, PRD-level revision,
  meta-template optimization.

Each speed has its own Bus topic namespace (`gamma.*`, `theta.*`,
`delta.*`). Cross-speed Pulses are explicit — the orchestrator at
Theta publishes `delta.plan.revision.requested` when accumulated Gamma
errors exceed threshold. This is Beer's *algedonic signal*: a
cross-layer alarm that bypasses the normal hierarchy when the lower
layer is failing. The Pulse model makes algedonic signals trivial.

## 5. Prediction error as a first-class metric

### 5.1 The `prediction_error` axis

Add a seventh Pulse topic family: `prediction.error.*`. Every predictor
publishes to it. Every learner subscribes. The TUI F4 tab grows a
"Prediction Error" sub-view showing:

- Per-operator calibration curves (predicted vs actual)
- EMA of prediction-error magnitude per topic
- Drift detection (sudden spike = model of world broken; trigger Dreams)

### 5.2 Prediction-error drives attention

High-prediction-error regions of the Substrate are where the agent is
*learning most*. Dreams should prioritize them for consolidation;
Neuro should promote their Engrams to higher tiers; the orchestrator
should re-plan around them. The Bus makes this a one-liner: subscribe
to `prediction.error.high`, enqueue the lineage for deeper analysis.

This is the formal implementation of curiosity-driven learning (Oudeyer
& Kaplan 2007) and intrinsic motivation (Schmidhuber 2010): agents
preferentially attend to regions where their models are improvable.

### 5.3 Prediction error becomes the c-factor anchor

Collective prediction error — summed across all operators in a
collective — is a sharper c-factor proxy than any individual metric.
Groups that collectively predict better are more intelligent. This is
operationally measurable at every tick. (Expanded in doc 15.)

## 6. Exponentially scaling loops

Cybernetics defines a positive feedback loop by its *gain*: output that
amplifies its own input. Roko has three natural exponential loops
available once the Bus is first-class:

### 6.1 Agents teaching agents (meta-imitation)

When agent A's turn wins a hard gate and agent B's turn loses on the
same task, the Bus has:

- `agent.turn.completed` from A with verdict=pass
- `agent.turn.completed` from B with verdict=fail
- Both with the same parent task hash

A `CrossAgentLearningPolicy` subscribes, extracts (A's prompt + A's
trajectory, B's prompt + B's trajectory), stores the pair as a
`ContrastiveEngram`, and feeds it into the next round of
Composer-template mutation. B literally learns from A without either
agent noticing.

At N agents, this is N(N-1) pairwise learning streams. Exponential
surface area.

### 6.2 Playbooks-of-playbooks

Today: playbooks distil episodes into reusable patterns. Next tier:
**meta-playbooks** distil playbooks into strategies ("when the task is
X, use playbook P, but if the first gate fails try P'"). Next: **plays**
distil meta-playbooks into policies. Every tier is a Delta-speed
consolidation loop over the previous tier's output topic. The data
volume compresses by ~10x per tier; knowledge depth grows
exponentially.

### 6.3 Self-modeling and meta-Gate

The agent maintains a model of itself: "what kinds of tasks does this
agent succeed on; what's its current prediction-error trend". Periodically,
a `MetaGate` runs on the agent's model of itself: if the model's own
predictions about itself are wrong, trigger a deeper review. This is
second-order feedback: the agent's learning about its own learning.

Second-order feedback enables Minsky's "society of mind" at the
architectural layer: each agent is an "A-brain", and the MetaGate is
the "B-brain" watching it. (Minsky 1986, §6.)

## 7. Making the current learning subsystems read off the Bus

The existing three learners in `roko-learn` become trivial after the
refactor:

### 7.1 CascadeRouter (already a bandit)

Today it's called explicitly by the router. After: it subscribes to
`router.selection.made` and `router.selection.outcome`, updates its
internal bandit, publishes `router.weights.updated`. Same mechanism as
everything else. Decouples from the Router code.

### 7.2 EpisodeLogger

Today: called from orchestrate.rs. After: subscribes to
`agent.turn.completed` + `gate.verdict.emitted`, constructs Episodes
by correlating lineage, stores them. Orchestrator doesn't know it
exists. Fully decoupled.

### 7.3 ExperimentStore

Today: called from Composer when building prompts. After: subscribes to
`composer.invocation.started`, decides variant, publishes
`composer.variant.assigned`. The Composer subscribes back for its
template choice. Hit rates tracked by listening to Gate verdicts.

None of this is *necessary* before Phase C; the current wiring works.
But the refactored version makes it much easier to add the fourth,
fifth, sixth learner without more crate dependencies.

## 8. The meta-insight

Once the Bus exists, **any function of past Pulses is a learnable
signal**. This is a superset of almost everything in the RL/LLM
training literature, because:

- Supervised learning: subscribe to `(input, label)` Pulses.
- RL: subscribe to `(state, action, reward)` Pulses.
- Imitation: subscribe to `(expert_action)` Pulses.
- Self-play: two agents publish on separate topics; a Policy joins them.
- Curiosity: subscribe to `prediction.error.*`.
- Distillation: subscribe to big-model outputs, train small-model
  responses.

All of these collapse to the same primitive: *subscribe to topics,
build a policy, publish new Pulses*. The framework becomes the learning
algorithm, not a substrate that learning happens on top of.

## 9. Practical next step (Phase C.5)

After Phase C's subsystem migration, a two-week Phase C.5 would:

1. Add `prediction.*` and `outcome.*` topic families to
   `roko-core::topics`.
2. Wrap each of the six operators with a thin "record prediction"
   instrument that publishes on the prediction topic without changing
   the operator's signature.
3. Land a single `CalibrationPolicy` in `roko-learn` that subscribes to
   every prediction/outcome pair, maintains per-operator calibration
   state, and publishes updates.
4. Connect the TUI F4 tab to render live calibration.

After that, the system has a *complete* feedback nervous system. Every
subsequent learning feature is a new subscription.

## 10. The `CalibrationPolicy` sketch

The Phase-C.5 work collapses to roughly this structure:

```rust
// crates/roko-learn/src/calibration/mod.rs (new)
use roko_core::{Bus, Pulse, Topic, TopicFilter};
use std::collections::HashMap;

/// Per-operator calibration state, indexed by operator name.
#[derive(Default)]
pub struct CalibrationState {
    pub trials: u64,
    pub squared_error_sum: f64,       // for RMSE
    pub brier_sum: f64,               // for probabilistic predictions
    pub ema_error: f64,               // exponential moving avg
    /// Per-axis calibration curve bins (for Scorer's 7-axis case).
    pub axis_curves: Vec<CalibrationBin>,
}

pub struct CalibrationBin {
    pub predicted: f64,
    pub observed: f64,
    pub count: u64,
}

/// A single Policy that watches all predict/outcome pairs and
/// publishes calibration updates per operator.
pub struct CalibrationPolicy<B: Bus> {
    pub bus: std::sync::Arc<B>,
    /// Map from (operator_name, lineage_hint) → predicted value.
    /// Outcome Pulses close the loop by matching lineage_hint.
    pending: parking_lot::Mutex<HashMap<(String, ContentHash), PredPayload>>,
    state: parking_lot::Mutex<HashMap<String, CalibrationState>>,
    ema_alpha: f64,  // e.g. 0.02
}

impl<B: Bus> CalibrationPolicy<B> {
    pub async fn run(self: std::sync::Arc<Self>) {
        let filter = TopicFilter::Or(
            Box::new(TopicFilter::Glob("prediction.**".into())),
            Box::new(TopicFilter::Glob("outcome.**".into())),
        );
        let mut rx = self.bus.subscribe(filter).await.unwrap();
        while let Some(pulse) = rx.recv().await {
            if pulse.topic.as_str().starts_with("prediction.") {
                self.record_prediction(&pulse);
            } else if pulse.topic.as_str().starts_with("outcome.") {
                if let Some(update) = self.close_and_update(&pulse) {
                    let _ = self.bus.publish(self.emit_update(update)).await;
                }
            }
        }
    }

    fn record_prediction(&self, p: &Pulse) {
        let Some(operator) = p.source.component.strip_prefix("operator:") else { return };
        let Some(lineage) = p.lineage_hint.clone() else { return };
        let Some(payload) = PredPayload::from_body(&p.body) else { return };
        self.pending.lock().insert((operator.to_string(), lineage), payload);
    }

    fn close_and_update(&self, p: &Pulse) -> Option<CalibrationUpdate> {
        let operator = p.source.component.strip_prefix("operator:")?.to_string();
        let lineage = p.lineage_hint.clone()?;
        let pred = self.pending.lock().remove(&(operator.clone(), lineage))?;
        let truth = OutcomePayload::from_body(&p.body)?;
        let err = loss(&pred, &truth);
        let mut state = self.state.lock();
        let s = state.entry(operator.clone()).or_default();
        s.trials += 1;
        s.squared_error_sum += err * err;
        s.ema_error = s.ema_error * (1.0 - self.ema_alpha) + err * self.ema_alpha;
        update_axis_curves(&mut s.axis_curves, &pred, &truth);
        Some(CalibrationUpdate {
            operator,
            rmse: (s.squared_error_sum / s.trials as f64).sqrt(),
            ema_error: s.ema_error,
            trials: s.trials,
        })
    }

    fn emit_update(&self, u: CalibrationUpdate) -> Pulse {
        Pulse {
            seq: 0,
            topic: Topic::new(&format!("calibration.{}.updated", u.operator)),
            kind: roko_core::Kind::Metric,
            body: roko_core::Body::Json(serde_json::json!({
                "operator": u.operator,
                "rmse": u.rmse,
                "ema_error": u.ema_error,
                "trials": u.trials,
            })),
            emitted_at_ms: now_ms(),
            source: roko_core::PulseSource {
                component: "roko-learn:calibration".into(),
                agent_id: None,
            },
            lineage_hint: None,
            trace_id: None,
        }
    }
}
```

The operator itself just publishes predictions on a `prediction.*`
topic and listens to `calibration.<self>.updated` for weight updates.
No coupling between operators; all through the Bus. Adding a seventh
learner is writing one predict-publish and one subscribe-update pair.

## 11. Intrinsic motivation and where to send attention

Once `prediction.error.*` is a live signal stream, two concrete
behaviors become trivial to implement:

1. **Next-task scheduling biased by prediction error.** An
   `IntrinsicMotivationPolicy` subscribes to recent
   `prediction.error.*` Pulses, aggregates per-topic or per-domain,
   and publishes `attention.request` Pulses when a region's error is
   elevated. The orchestrator's plan generator honors these requests
   by prioritizing tasks that touch the high-error region.
2. **Dreams consolidation priority.** Dreams (Phase-2) wakes on
   `substrate.engram.stored` density, but it can *also* wake when
   `prediction.error.ema` crosses threshold — the regions where the
   system is confidently wrong are the regions where replay +
   consolidation pay off most.

Both are three- to five-lines-of-Rust additions. Both are direct
implementations of Oudeyer-Kaplan curiosity-driven learning and
Schmidhuber's compression-progress-as-intrinsic-reward. The runtime's
"interest" is formally where its error is highest, and that's
observable from the Bus.

## 12. Where this synergizes across the folder

Self-learning is the most pervasive primitive in the refactor — it
touches nearly every other refinement:

- **HDC (11)** makes per-operator predictions *comparable*: the
  prediction vector and the outcome vector both fingerprint into the
  same HD space, so `Similarity(fp(predicted), fp(actual))` is a
  universal error signal.
- **Demurrage (12)** is reinforced by the `ReinforceKind::Surprised`
  signal that `prediction.error.high` produces. Surprising Engrams
  stay warm longer; unsurprising ones fade.
- **Heuristics (14)** have a built-in calibration field — they are
  the highest-value case for per-operator predict/outcome pairs. A
  heuristic whose falsifier fires is exactly a prediction error.
- **c-factor (13)** treats cross-agent peer-prediction error as the
  social-perceptiveness metric. The calibration infrastructure is
  the plumbing for the c-factor measurement.
- **Replication ledger (16)** is calibration applied to paper claims
  — "we predict this paper's effect replicates; we observed Y." The
  ledger is a `CalibrationPolicy` instance specialized to paper
  lineage.
- **Competitive moat (18)** §2.1 — the *composition* of these loops
  is the moat. Any single loop is a research project; all of them
  through one Bus + Substrate is an architecture.

--- END 10-self-learning-cybernetic-loops.md ---

# Batch REF10 — Self-learning cybernetic loops across learning + heartbeat

**Refinement source**: `tmp/refinements/10-self-learning-cybernetic-loops.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- `docs/05-learning/` — describe the predict-publish-correct loop; per-operator calibration; CalibrationPolicy. New file(s) as needed.
- `docs/00-architecture/11-dual-process-and-active-inference.md` — FEP-as-literal wording; prediction/outcome Pulse framing.
- `docs/00-architecture/16-autocatalytic-and-cybernetics.md` — Bus as feedback nervous system.
- `docs/16-heartbeat/11-active-inference-state-space.md` — prediction.*/outcome.* topic family reference.

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/10-self-learning-cybernetic-loops.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.


## Required vocabulary (verify)

The verify step greps for: `prediction.?error|active inference`

At least one changed file must mention it (case-insensitive) before the
batch is considered complete.

## Cross-references

Dependents — downstream refinements assuming this one has propagated
into docs: REF13, REF14

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
- Commit ready with message `refinements(REF10): Self-learning cybernetic loops across learning + heartbeat`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
