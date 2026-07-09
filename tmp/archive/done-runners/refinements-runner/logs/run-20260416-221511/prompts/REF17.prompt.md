# Refinements Batch REF17

Run id: run-20260416-221511
Attempt: 1
Model: gpt-5.4
Reasoning: high
Refinement source: /Users/will/dev/nunchi/roko/roko/tmp/refinements/17-plugin-extension-architecture.md
Target docs (candidates): docs/18-tools/ docs/12-interfaces/ docs/00-architecture/15-crate-map.md

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

Suggested parallel split for batch `REF17`:

- worker: add/update files under `docs/18-tools/` or `docs/12-interfaces/`
  covering the five-tier plugin SPI, manifests, sandboxes.
- worker: update `docs/00-architecture/15-crate-map.md` with roko-spi and
  related new crates.

## Canonical refinement source

This is the verbatim refinement proposal that this batch must propagate
into `docs/`. Treat it as the authoritative source. Do not edit this
file; only edit docs under `docs/`.

--- BEGIN 17-plugin-extension-architecture.md ---

# Plugin & Extension Architecture

> **TL;DR**: Roko's value as a platform — rather than a product —
> depends on how easy it is for a third-party to contribute a
> Substrate, a Gate, a Scorer axis, a Composer template, a tool, an
> MCP server, or an entire Role. This doc proposes a layered SPI
> with five distinct extension points, each with a stable trait
> contract, a discovery mechanism, and a sandbox story. The goal:
> someone outside the project can ship a valuable extension in an
> afternoon, and we can trust it without auditing it line-by-line.

> **For first-time readers**: This doc defines how outsiders contribute
> to Roko without forking it. The five tiers range from "pure data"
> (markdown prompt template) to "arbitrary code in a WASM sandbox."
> Each tier has a risk/power trade-off and a matching sandbox. The key
> insight: the plugin tier should be chosen by the plugin's *power
> needs*, and Roko's safety layer should match the tier automatically.
> Read 22 (dev UX) and 25 (domain profiles) alongside — they are the
> two biggest consumers of this SPI.

## 1. Current state

Extension points today are implicit:

- Traits exist in `roko-core` but don't have companion examples in
  `examples/` showing third-party implementations.
- MCP servers are discoverable but invoking an arbitrary MCP means
  changing `roko.toml`.
- Tools live in `roko-std` and are hard-coded; adding one means a
  PR against core.
- Roles are in `roko-compose/src/templates/` — template files
  mixed with builder code.

Nothing is *wrong* with any of these. But nothing is *designed for
external consumption* either. Every extension touches core.

## 2. The five extension tiers

### Tier 1 — **Templates and prompts** (lowest barrier)

Role prompts, system-prompt layers, tool descriptions. Should be
pure data — TOML or Markdown with front-matter. Discoverable by a
glob under `plugins/prompts/**`. Safe by construction; no code
execution.

### Tier 2 — **Configuration profiles**

Bundles of settings: a "Python shop" profile, a "Rust OSS" profile,
a "frontend team" profile. Layer on top of `roko.toml`.
Discoverable by `plugins/profiles/**`.

### Tier 3 — **Declarative tools and MCPs**

Tools described by JSON schema + invocation spec (subprocess, HTTP,
MCP). No Rust needed. The agent sees them exactly like core tools.
Discoverable by `plugins/tools/**`. Safe inside the existing tool
sandbox.

### Tier 4 — **Native trait implementations**

Rust crates that implement one of the six kernel traits (Substrate,
Bus, Scorer, Gate, Router, Composer, Policy). Compiled into the
binary or loaded as a cdylib with the ABI stable. For advanced
extensions — custom vector stores, custom gate types, custom
routing strategies.

### Tier 5 — **WASM sandboxed extensions**

Extensions that want to execute logic but that we don't trust to
run in-process. Targets wasm32-wasi, imports a host SPI, gets
memory-limited and deterministic. Slightly slower, much safer.

Each tier is a different risk/power trade-off. A plugin author
picks the lowest tier they can get away with.

## 3. Extension SPI at a glance

```rust
pub trait Extension {
    /// Stable identifier: "org.example.my_gate"
    fn id(&self) -> &str;

    /// Semver of this extension.
    fn version(&self) -> Version;

    /// Declared capabilities: which trait(s) this implements.
    fn capabilities(&self) -> &[Capability];

    /// Declared permissions required (network, files, tools).
    fn permissions(&self) -> &Permissions;

    /// Health check — called on load and periodically.
    fn health(&self) -> Health;
}

pub enum Capability {
    Substrate(SubstrateKind),  // content-addressed, ephemeral, cold, ...
    Bus(BusKind),              // in-memory, persistent, cluster
    Scorer { axes: Vec<ScoreAxis> },
    Gate { rungs: Vec<GateRung> },
    Router { strategy: RouterStrategy },
    Composer { template_classes: Vec<TemplateClass> },
    Policy { kind: PolicyKind },
    Tool { schema: JsonSchema },
    Role { name: String, description: String },
}
```

An extension registers by placing a manifest at a known path; the
loader resolves and dispatches.

## 4. Declarative tool manifest (Tier 3)

The most important tier for ecosystem growth. Tool plugins should
be *purely declarative*:

```toml
# plugins/tools/cargo-udeps.toml
[tool]
id          = "cargo.udeps"
version     = "0.1.0"
description = "Find unused dependencies in a Rust project"

[tool.schema]
name        = "cargo_udeps"
description = "Detect unused deps in Cargo.toml"
parameters  = { workspace_root = { type = "string" } }

[tool.invoke.subprocess]
cmd         = "cargo"
args        = ["+nightly", "udeps", "--workspace"]
cwd         = "{{workspace_root}}"

[tool.safety]
role_allow  = ["researcher", "implementer"]
network     = false
files       = ["{{workspace_root}}/**"]
timeout_ms  = 300000
```

The Roko loader sees this, validates, and exposes `cargo_udeps` to
any role in `role_allow`. No PR against core. Publishable to a
registry. **This is the single biggest ergonomics win.**

## 5. Versioning and ABI stability

Tier 4 (native traits) has a hard versioning problem — Rust doesn't
have a stable ABI. Two solutions, used together:

1. **cdylib bridge**: a small `roko-extension-abi` crate defines a
   narrow C-FFI layer (version struct, vtable, opaque pointers).
   Native extensions compile against this. Semver bumps require
   recompilation but don't break data.
2. **In-tree extensions** as a secondary default: users can drop a
   crate into `./plugins/native/` and the Roko binary rebuilds
   itself with it included. Cargo handles the rest. This is the
   ergonomics-friendly path for project-local extensions.

Tier 5 (WASM) sidesteps all of this. wasm-bindgen + wasi-preview2
give a stable ABI by construction. This is likely where we push
serious third-party development.

## 6. Discovery, not configuration

Plugin configuration should be minimal. The loader walks
`plugins/**`, reads manifests, validates. A new plugin should be
usable without editing `roko.toml`. Disabling a plugin is
`plugins/<id>/disabled` or `roko plugin disable <id>`.

The `roko plugin` CLI:

```bash
roko plugin list                    # installed
roko plugin search <query>          # from registry
roko plugin install <id>            # into ./plugins
roko plugin uninstall <id>
roko plugin enable <id>
roko plugin disable <id>
roko plugin info <id>
roko plugin audit                   # permission review
```

## 7. The registry

A Roko Plugin Registry — `plugins.roko.dev` or similar — modeled on
crates.io but narrower:

- Plugins publish with a signed manifest.
- Each plugin lists required tier, permissions, and a health script.
- Reviews come from actual deployments: a plugin that has been
  active in 50 deployments for 30 days gets a verified badge.
- Security issues surface through a CVE channel.

This is a Phase-2 move. Phase-1 is a github-based mechanism:
plugins in public repos, `roko plugin install <github-url>`, trust
based on signatures.

## 8. Sandboxing model

For each tier:

| Tier | Sandbox | Notes |
|---|---|---|
| 1 | None needed | pure data |
| 2 | None needed | pure data |
| 3 | Existing tool safety layer | subprocess / MCP respects role_allow, files, network |
| 4 | Rust process isolation | honor system's Linux namespaces / macOS seatbelt |
| 5 | wasm capability sandbox | imports only what host SPI exposes |

The safety layer in `crates/roko-agent/src/safety/` already handles
tier 3. Tier 5 needs a new subcrate — `roko-wasm-host` — that
implements the host interface and enforces limits.

## 9. Extension invariants the Roko core must honor

For the plugin story to work, the core must:

1. **Never break trait contracts without a semver bump.**
2. **Never change persistent data formats without a migration.**
3. **Always emit events plugins can subscribe to** (Bus).
4. **Always expose a read-only Substrate view** (no hidden state
   that plugins can't see).
5. **Always report its own version and capabilities** so plugins
   can feature-detect.

These are cheap to commit to now, expensive to retrofit later.

## 10. Example flows

### 10.1 Adding a company-specific gate

A team wants a gate that checks "PRs always include a `Closes #N`
reference." They write a tier-4 Rust crate implementing `Gate`,
drop it in `./plugins/native/gates/`, `roko plugin enable`. The
next run, the gate is part of the pipeline. One afternoon of work.

### 10.2 Adding a domain-specific tool

A team using Kubernetes wants `kubectl_apply` with policy
enforcement. They write a tier-3 TOML manifest wrapping a shell
script. Drop into `./plugins/tools/`. Roles that need it list it
in `role_allow`. Done.

### 10.3 Adding a new Role

A team wants a "Compliance Reviewer" role with specific templates
and a custom scorer axis. They write:

- `plugins/prompts/compliance_reviewer.md` (tier 1)
- `plugins/tools/compliance_check.toml` (tier 3)
- `plugins/native/scorers/compliance_axis` (tier 4)

All three get discovered and wired. No core PR.

## 11. Why this is a moat

An agent framework's long-term value is measured in its
ecosystem, not its core code. OpenAI's API is valuable because of
what people build on it. Rust is valuable because of its crates.
The Linux kernel is valuable because of its drivers.

Roko's moat, five years out, is not the Substrate or the Bus; it's
the *thousand company-specific gates, tools, and roles* that got
written because the SPI was stable and the risk/power tiers were
well-chosen. Investing now in a clean extension story compounds
(see `15-exponential-scaling.md` §2.7).

## 12. Implementation staging

- **Stage A** (weeks 1–2): Tier 3 tool manifests + discovery.
  Biggest immediate value.
- **Stage B** (weeks 2–3): Tier 1 + Tier 2 prompt/profile plugins.
  Docs-heavy but mostly data.
- **Stage C** (weeks 3–5): Tier 4 ABI bridge + `roko plugin` CLI.
- **Stage D** (month 2+): Tier 5 WASM host.
- **Stage E** (month 3+): Registry.

After Stage A, "install a plugin" is a real user action. After
Stage D, serious third-party development is safe. After Stage E,
we have a real ecosystem.

## 13. WASM host surface (Tier 5 in depth)

The tier-5 WASM sandbox is the tier most likely to unlock third-party
development at scale. A few specifics on what the host imports look
like:

```rust
// roko-wasm-host/src/abi.rs (new crate)
// All function signatures WASM-stable. Extensions compile against these.

pub mod host {
    /// Read an Engram by hash. Returns length; caller pre-allocates buf.
    pub fn engram_get(hash_ptr: u32, hash_len: u32, buf_ptr: u32, buf_cap: u32) -> i64;

    /// Publish a Pulse. Returns the sequence number or an error code.
    pub fn bus_publish(pulse_ptr: u32, pulse_len: u32) -> i64;

    /// Subscribe to a topic filter. Returns a subscription handle.
    pub fn bus_subscribe(filter_ptr: u32, filter_len: u32) -> i32;

    /// Receive the next Pulse for a subscription. Returns length;
    /// zero = no Pulse ready; negative = error.
    pub fn bus_recv(sub_handle: i32, buf_ptr: u32, buf_cap: u32) -> i64;

    /// HDC similarity query against Substrate.
    pub fn substrate_query_similar(fp_ptr: u32, radius_bits: u32, limit: u32,
                                   out_ptr: u32, out_cap: u32) -> i64;

    /// Typed logging back to the host.
    pub fn log(level: u32, msg_ptr: u32, msg_len: u32);

    /// Request a wall-clock timestamp (milliseconds since epoch).
    pub fn now_ms() -> i64;
}
```

Everything the extension does goes through these imports. No file
system, no network, no arbitrary syscalls. The host enforces:

- CPU budget per call (default 100 ms wall clock).
- Memory limit (default 64 MB per instance).
- Pulse publish rate limit (100 pulses/sec default; tunable).
- Substrate query limit (100/sec default).
- Pulse body size limit (64 KB default).

Violations kill the instance. Host publishes `plugin.violation` Pulses
so operators see what happened.

## 14. Permission manifests (all tiers)

Every plugin declares its required permissions in its manifest. The
host honors declared permissions and refuses to grant anything
outside them:

```toml
# plugins/native/my_gate/manifest.toml
id = "org.example.my_gate"
version = "0.2.1"
tier = "native"
capabilities = [ "Gate{rungs=[\"style\"]}" ]

[permissions]
network     = false
files_read  = ["**/*.rs"]
files_write = []
bus_topics_subscribe = ["gate.verdict.emitted"]
bus_topics_publish = ["gate.failed.org.example.my_gate"]
substrate_kinds_read  = ["GateVerdict"]
substrate_kinds_write = ["GateVerdict"]
hdc = false
env_vars = []
```

Manifests enable static analysis: `roko plugin audit` can report
any plugin that requests network, file-write, or broad bus access,
and the operator can decide before installing. This supports the
safety story in `32-safety-sandbox-provenance.md`.

## 15. Dogfooding the SPI

A good test of the SPI: can Roko's own built-in tools, gates, and
scorers be *rewritten* against the same SPI the plugin ecosystem
uses? If not, the SPI is hiding functionality that third parties
will inevitably demand.

Recommendation: after Stage D, port three built-ins (one tool, one
gate, one scorer) to the plugin SPI and dogfood. This catches SPI
gaps while the team still remembers why the internal APIs exist.

## 16. Cross-references

- Domain profiles from `25-domain-specific-agents.md` are the
  largest consumer of Tier-2 profile bundles.
- Developer UX from `22-developer-ux-rust.md` §2.3 covers what
  Tier-4 native extensions see from a Rust author's side.
- Web UI custom tiles (`29-web-ui-architecture.md` §11.1) use the
  plugin mechanism for front-end extensions.
- Custom projections in StateHub (`26-statehub-rearchitecture.md`
  §12) are Tier-4 extensions registering against the projection
  trait.
- The safety story that Tier-3 and Tier-5 rely on lives in
  `32-safety-sandbox-provenance.md`.
- Marketplace / registry items will also appear in
  `33-observability-telemetry.md` — install-count, version uptake,
  security-issue reports surface as dashboard metrics.

--- END 17-plugin-extension-architecture.md ---

# Batch REF17 — Plugin/extension five-tier SPI across tools + interfaces

**Refinement source**: `tmp/refinements/17-plugin-extension-architecture.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- `docs/18-tools/` — five-tier SPI (prompts, profiles, manifests, native, WASM), tool manifests, MCP integration.
- `docs/12-interfaces/` — `roko plugin` CLI surface.
- `docs/00-architecture/15-crate-map.md` — roko-spi crate added.

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/17-plugin-extension-architecture.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.


## Required vocabulary (verify)

The verify step greps for: `plugin|five.?tier|SPI`

At least one changed file must mention it (case-insensitive) before the
batch is considered complete.

## Cross-references

Dependents — downstream refinements assuming this one has propagated
into docs: REF18, REF20, REF25, REF29, REF31, REF32

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
- Commit ready with message `refinements(REF17): Plugin/extension five-tier SPI across tools + interfaces`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
