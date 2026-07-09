# Refinements Batch REF15

Run id: run-20260416-221511
Attempt: 1
Model: gpt-5.4
Reasoning: high
Refinement source: /Users/will/dev/nunchi/roko/roko/tmp/refinements/15-exponential-scaling.md
Target docs (candidates): docs/00-architecture/16-autocatalytic-and-cybernetics.md docs/00-architecture/30-cross-pollination-innovations.md docs/20-technical-analysis/ docs/13-coordination/10-exponential-flywheel.md

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

Suggested parallel split for batch `REF15`:

- worker: update `docs/00-architecture/16-autocatalytic-and-cybernetics.md` with
  the seven compounding loops enumeration.
- worker: update `docs/00-architecture/30-cross-pollination-innovations.md`
  with the network-effect claims.
- worker: update files under `docs/20-technical-analysis/` with the superlinear
  scaling story.
- worker: update `docs/13-coordination/10-exponential-flywheel.md` if present.

## Canonical refinement source

This is the verbatim refinement proposal that this batch must propagate
into `docs/`. Treat it as the authoritative source. Do not edit this
file; only edit docs under `docs/`.

--- BEGIN 15-exponential-scaling.md ---

# Exponential Scaling Patterns

> **TL;DR**: Roko should be designed so that performance, capability,
> and quality improve *superlinearly* with accumulated usage,
> deployment count, and connected data. This doc identifies seven
> specific mechanisms already or nearly in the codebase that exhibit
> compounding returns and shows how to tune each for maximum
> positive feedback. The goal: every unit of usage should make the
> next unit cheaper, faster, and better.

> **For first-time readers**: This doc is the "why does all the previous
> work compound?" story. Each of the seven loops builds on a specific
> earlier doc — demurrage (12), heuristics (14), HDC (11), c-factor
> (13), playbooks, commons (14 §10), plugins (17). Read this as a map of
> which features depend on which others to produce *superlinear* rather
> than *linear* returns. Read 31-synergy-integration-map.md alongside
> for the full cross-weave.

## 1. Why linear returns are the default failure

Most agent frameworks exhibit **diminishing** returns:

- Each new agent adds marginal value until the coordination cost
  exceeds the added throughput.
- Each new memory item dilutes retrieval quality.
- Each new tool adds context-window pressure.
- Each new user adds support burden without proportionate benefit
  to other users.

Roko's architecture has the ingredients to flip several of these
into *increasing* returns, but only if we deliberately connect the
feedback loops. This doc enumerates them.

## 2. The seven compounding loops

### 2.1 Demurrage-weighted retrieval (sub-linear → super-linear)

Naive memory grows O(n) and retrieval quality degrades. With
demurrage (`12`), memory grows but *attention-weighted* memory is
capped — so retrieval quality grows in *effectively-indexed* memory
which is bounded. Compounding mechanism: more usage → better
reinforcement signal → better calibration of demurrage rates →
sharper effective index.

Scaling law: retrieval quality ∝ log(trials) × calibration_quality(trials),
which superlinearly improves as trials increase.

### 2.2 Heuristic calibration (Bayesian compounding)

Every episode is a trial for dozens of heuristics. Confidence
intervals tighten as O(1/√n). The value of a well-calibrated
heuristic in a downstream decision goes as log-odds, which *depends
linearly on calibration quality*. Combined, the value extracted per
episode grows with √n *per heuristic* and with the count of
applicable heuristics *across* episodes — a multiplicative effect.

### 2.3 HDC codebook cleanup (quantization returns)

Every new episode, gate result, and heuristic adds to the HDC
codebook. HDC's *cleanup* operation snaps a noisy fingerprint to the
nearest codebook entry. Cleanup quality improves with codebook size
*up to* the capacity of the space (which is enormous at 10,240
bits). So for any foreseeable scale, every new episode makes every
subsequent retrieval more likely to hit a clean match. Payback
accelerates.

### 2.4 c-factor feedback

From `13`: teams with measured high c deliver higher-quality output.
High-quality output produces higher-quality reinforcement signals.
Higher-quality reinforcement improves heuristic calibration (2.2)
and demurrage rate tuning (2.1). Better priors and memory produce
higher c. **This is a three-loop reinforcement**: c → quality →
learning → c.

### 2.5 Playbook distillation (meta-learning)

Episodes are distilled into playbooks. Playbooks themselves can be
distilled into meta-playbooks ("when distilling a cluster of
episodes, the presence of gate failures suggests pre-condition
emphasis"). This is learning about learning. Each level of
distillation is cheaper per episode (because the level below is
already compressed) and more transferable (because it's more
abstract). Value per distillation-unit grows with depth.

### 2.6 Cross-deployment heuristic commons

If heuristics can be imported across deployments (`14` §10), then
every deployment contributes to a commons. The value of the commons
goes roughly as O(n) in deployment count but the cost to each
deployment is O(1). So the marginal value of the Nth deployment to
itself is O(1) but the marginal value to *everyone else* is also
O(1), giving a total system value of O(n²). This is a metcalfe's-law
effect at the heuristic level.

### 2.7 Plugin ecosystem (two-sided)

Once plugins exist (`17`), each new plugin increases the value of
Roko to users who need that capability, and each new user increases
the value of building a plugin. Classic two-sided network effect.
The magnitude depends on the quality of the plugin interface — bad
interfaces produce weak network effects because plugins leak
complexity to users.

## 3. Measuring compounding

We need instruments. Three scaling dashboards:

```
┌─ Learning Curves ─────────────────────────────────────┐
│ Retrieval quality vs episode count: ╱╱╱ (log-slope)   │
│ Heuristic calibration CI width:     ╲╲╲              │
│ c-factor trend:                     ╱╱                │
│ Codebook cleanup hit rate:          ╱╱╱               │
└───────────────────────────────────────────────────────┘
```

If any line flatlines, we have a blocker somewhere. Flatlining
retrieval quality means the feedback to demurrage isn't working. A
flatlining heuristic CI width means the Calibrator isn't getting
fresh trials.

## 4. Failure modes of compounding

### 4.1 Echo chambers

A tight positive loop can reinforce wrong beliefs. Countermeasures
are already in `13` (WisdomGate, outsider injection) and `14`
(challenger worldviews).

### 4.2 Reward hacking

If the Policy optimizes for c directly, it can achieve high c by
routing only to easy tasks. Countermeasure: **c is a covariate, not
the objective**. The objective is gate pass-rate on a task sampled
by difficulty. c is a measured property that correlates with — but
does not replace — outcome quality.

### 4.3 Premature convergence

A heuristic that hits 95% confidence early and then prevents its
own refutation by influencing which situations it's tested against.
Countermeasure: **importance sampling** — the Calibrator
occasionally runs heuristics on situations that *shouldn't* match,
to probe the boundary. Classic bandit exploration bonus applied to
a prior rather than an arm.

### 4.4 Substrate bloat

Infinite retention without demurrage blows out disk. Demurrage
(`12`) is the structural answer, but rates need tuning so cold tier
actually gets used. Observability: balance histogram.

## 5. Net-new superlinear primitives

These don't exist in other agent frameworks and are architecturally
possible only in Roko:

### 5.1 Prediction markets on heuristics

Agents can *stake* confidence on a heuristic's next outcome. Stakes
are balance-credits (see `12`). Correct predictions earn balance;
incorrect lose it. The aggregate stake becomes a secondary
confidence signal alongside the Bayesian calibration. This is a
Robin Hanson prediction-market design mapped onto the Bus: an
internal market for *truth* about the system's own priors.

### 5.2 Compositional tool-curricula

Every successful tool sequence is an Engram. HDC binding of tool
sequences gives us a compositional space of "plans of tools." New
plans are generated by HDC arithmetic: plan_for_new_task ≈
bundle(similar_plan_1, similar_plan_2) cleaned to the codebook. This
is Plotkin/MML/analogical reasoning, but fast and content-addressed.

### 5.3 Self-modeling

Roko observes its own latencies, its own failure rates, its own c.
These are Engrams too. A *meta-agent* can read those Engrams and
propose changes to policy parameters. The Bus carries a
`system.self` topic. This is John Holland's "internal models"
applied to the runtime itself.

## 6. Compounding as a product claim

"Every week your Roko gets faster, more accurate, and more
collaborative *on your codebase*."

This is a much stronger claim than "we have learning." It's
*provable* from the measurements above, and the failure modes are
known and instrumented. No other agent framework makes this claim
because none of them have the substrate to back it up.

## 7. The Phase-2 super-loops

Three additional compounding loops unlock when Phase 2 lands:

- **Witness-signed heuristic commons**: cross-deployment heuristics
  that carry chain witness signatures gain exponential trust with
  more signatures.
- **Dream-consolidation compression**: offline consolidation
  (`roko-dreams`) re-reads episodes during idle time, re-distilling
  them with current knowledge. Retroactive learning — old episodes
  produce *new* heuristics when viewed through current priors.
- **Agent-chain specialization**: roles specialize through chains
  of interactions; specialization stratifies the agent population
  into a functional ecosystem.

Phase 2 is where Roko gets *weirdly good* — not just incrementally
better but qualitatively different. These loops require the
two-fabric substrate from `02` and `03` to exist.

## 8. What to ship to enable compounding

Priority order for superlinear returns:

1. **Demurrage + reinforcement** (`12`). Enables 2.1 and supports 2.2.
2. **Heuristic type + Calibrator** (`14`). Enables 2.2 and 2.6.
3. **HDC-on-every-Engram** (`11`). Enables 2.3 and 5.2.
4. **c-factor metrics** (`13`). Enables 2.4 and 2.7 measurement.
5. **Plugin SPI** (`17`). Enables 2.7 network effect.
6. **Dream-cycle** (Phase 2). Enables retroactive compounding.

Steps 1–4 are a few weeks of focused work and unlock most of the
superlinear returns. Steps 5–6 require more build but produce the
"competitive moat" defensibility.

## 9. The single most important metric

If we had to pick one scaling KPI it would be:

**Mean time to first successful PR on a new codebase.**

This metric depends on all seven loops: good priors (2.2, 2.6), good
retrieval (2.1, 2.3), good collaboration (2.4), good compression
(2.5), good tooling (2.7), good self-awareness (5.3). It should drop
*steeply* over the first few weeks of a deployment and keep dropping
at a decreasing-but-positive rate indefinitely.

Plot this, optimize this, and the product tells its own story.

## 10. Secondary scaling KPIs

Beyond the single headline metric, track these to detect which loop
is doing the work (or failing):

| KPI | Loop it measures | Expected curve |
|---|---|---|
| Median tokens per task (by task difficulty bucket) | 2.1 + 2.3 + 2.5 | Monotonic decrease |
| % of Composer prompts hitting HDC-clean cache | 2.3 | Asymptote near 1 |
| Mean calibration CI width per heuristic | 2.2 | Decrease with n |
| % of heuristics sourced from commons | 2.6 | Increase then stabilize |
| c-factor on randomly-sampled cohorts | 2.4 | Stable or rising |
| Dream-cycle retroactive improvements / week | 7 (Phase 2) | Grows with corpus |
| Plugin count (unique) | 2.7 | Linear in t |
| Plugin unique users | 2.7 | Superlinear once flywheel turns |
| Mean time per task by task class | Composite | Monotonic decrease |
| First-task-after-install → success minutes | 2.6 commons bootstrap | Decreases over commons growth |

The first KPI is user-facing; the rest are operator KPIs. Expose
them as part of `33-observability-telemetry.md`.

## 11. Anti-metrics — what should *not* grow

Three numbers should stay flat or shrink even as usage grows.
These are the "we're not cheating" checks:

- **Episode count in warm tier** — if demurrage is working, warm-tier
  episode count reaches a steady state, it doesn't grow unbounded.
- **Heuristic count with confirmations < 3** — new hypotheses enter,
  but unconfirmed ones fade. If this grows unbounded, the Calibrator
  isn't probing them fast enough.
- **Mean lineage depth per response** — deep lineage is fine when
  it's load-bearing; growing lineage without growing answer quality
  means the Composer is hoarding context for no reason.

If any of these blow out, pause feature work and tune rates (per
12 §14).

## 12. Load-bearing assumption: real workloads, not synthetic benchmarks

The "superlinear returns" claim depends on real workloads with
persistence across sessions. A benchmarking suite that resets state
between runs will see none of the compounding. Two implications:

1. **Evals must span sessions.** Each benchmark task should be
   attempted N times over M days with the agent's substrate
   preserved. Measure the slope of `time_to_solve` over N trials.
2. **Heuristic commons contribution is a variable.** Evaluate the
   same agent with and without commons access. The gap is the
   commons' contribution to the compounding.

Without these guardrails, the product claim in §6 is an assertion,
not an observation. With them, the claim becomes falsifiable — which
is the replication-ledger ethos from `16-research-to-runtime.md`
applied to our own product promises.

## 13. The kill switches

If compounding goes wrong, the operator needs an emergency stop:

- `roko attention reset` — zero all balances back to starting value.
- `roko heuristic retire --confidence-below 0.3` — sweep out weakly
  calibrated heuristics.
- `roko substrate freeze --older-than 90d` — manually graduate old
  Engrams to cold tier.
- `roko commons opt-out` — stop importing from or exporting to the
  shared commons.
- `roko experiments pause` — freeze prompt-experiment variant rotation.

Each is reversible. Each is surfaced in the TUI Settings tab. Having
them documented and tested *before* they're needed is cheap insurance.

--- END 15-exponential-scaling.md ---

# Batch REF15 — Exponential scaling loops across autocatalytic + technical-analysis

**Refinement source**: `tmp/refinements/15-exponential-scaling.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- `docs/00-architecture/16-autocatalytic-and-cybernetics.md` — seven compounding loops enumerated.
- `docs/00-architecture/30-cross-pollination-innovations.md` — network-effect claims.
- `docs/20-technical-analysis/` — superlinear scaling claim backed by KPIs and anti-metrics.
- `docs/13-coordination/10-exponential-flywheel.md` — align with the seven-loops framing.

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/15-exponential-scaling.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.


## Required vocabulary (verify)

The verify step greps for: `compounding|superlinear|exponential`

At least one changed file must mention it (case-insensitive) before the
batch is considered complete.

## Cross-references

Dependents — downstream refinements assuming this one has propagated
into docs: REF18

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
- Commit ready with message `refinements(REF15): Exponential scaling loops across autocatalytic + technical-analysis`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
