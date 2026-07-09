# Refinements Batch REF12

Run id: run-20260416-221511
Attempt: 1
Model: gpt-5.4
Reasoning: high
Refinement source: /Users/will/dev/nunchi/roko/roko/tmp/refinements/12-knowledge-demurrage.md
Target docs (candidates): docs/00-architecture/04-decay-variants.md docs/00-architecture/18-decay-tier-matrix.md docs/00-architecture/25-attention-as-currency.md docs/06-neuro/ docs/05-learning/

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

Suggested parallel split for batch `REF12`:

- worker: update `docs/00-architecture/04-decay-variants.md` to introduce
  demurrage superseding decay; document balance/reinforcement/thaw.
- worker: update `docs/00-architecture/18-decay-tier-matrix.md` with the
  new tier graduation rules.
- worker: update `docs/00-architecture/25-attention-as-currency.md` with
  the demurrage economic framing.
- worker: update `docs/06-neuro/` and `docs/05-learning/` where playbook
  freshness / episode retention is discussed.

## Canonical refinement source

This is the verbatim refinement proposal that this batch must propagate
into `docs/`. Treat it as the authoritative source. Do not edit this
file; only edit docs under `docs/`.

--- BEGIN 12-knowledge-demurrage.md ---

# Knowledge Demurrage: Economic Memory

> **TL;DR**: Borrow Silvio Gesell's *demurrage* idea from economics and
> apply it to memory. Every Engram carries a *holding cost* that decays
> its weight unless it is actively used, cited, or reinforced. The
> result: memory that stays fresh, playbooks that don't ossify,
> worldviews that can't petrify, and a system that preferentially
> surfaces *currently useful* knowledge rather than *historically
> cached* knowledge. This is the opposite of a cache — it's an
> attention economy with gravity.

> **For first-time readers**: Demurrage is a concept from Silvio Gesell's
> 1916 economic theory: money that *costs* its holder to keep idle, so it
> circulates faster. Applied to memory: every stored artifact pays a tiny
> "tax" per unit time; *usage* refunds the tax; unused knowledge fades to
> cold storage. Contrast with LRU (wall-clock only), TTL (arbitrary
> expiry), and current Roko decay (time-based, not reinforcement-based).
> This doc is where memory gets an economy. Read 11 first for the HDC
> fingerprint that powers the novelty bonus.

## 1. The problem with indefinite retention

Roko already has mild decay: `Engram.decay: f64` starts at 1.0 and is
reduced by the GC pass in `roko-fs`. But decay today is:

- **Time-based only**, not usage-based
- **Applied at GC time**, not continuously
- **Non-compounding** — recent decay doesn't accelerate further decay
- **Invisible to the Scorer** — decay doesn't downweight candidates

This leads to three failure modes that show up in any long-running
agent system:

1. **Playbook petrification**: a playbook that worked once is preserved
   forever, even after the codebase drifts underneath it.
2. **Stale consensus**: agents converge on a shared but out-of-date
   belief because nothing punishes its age.
3. **Archive paralysis**: episode retrieval blends 10,000 old episodes
   with 10 recent ones; the Router loses signal.

Biological memory solves this with *use-it-or-lose-it* — synapses that
aren't activated decay faster. We can do the same, but make it
explicit and measurable.

## 2. Demurrage, not decay

**Decay** is exogenous: "it's been 30 days, halve the weight."
**Demurrage** is endogenous: "you pay a tax on holding; use lowers the
tax, non-use compounds it."

The economic analogy matters because it produces the right incentive
gradient: *information that is useful to multiple subscribers stays
alive; information that nothing cares about fades, making room for new
information*. It is a market for attention, with a carrying cost.

Proposed addition to Engram:

```rust
pub struct Engram {
    // ... existing fields ...

    /// Balance of attention-credit held by this Engram.
    /// Starts at 1.0 at creation. Demurrage subtracts
    /// a tick per unit time; reinforcement adds credit.
    pub balance: f64,

    /// Accumulated tax paid (for observability). Monotonic.
    pub demurrage_paid: f64,

    /// Last time `balance` was touched by demurrage or reinforcement.
    pub last_touched_at: Timestamp,
}
```

And a new trait the Substrate implements transparently:

```rust
pub trait Demurrage {
    /// Charge demurrage since `last_touched_at`. Returns new balance.
    fn charge(&mut self, engram: &mut Engram, now: Timestamp) -> f64;

    /// Reinforce an engram (read, cite, successful use). `kind`
    /// encodes *why* so we can tune rates per reason.
    fn reinforce(&mut self, engram: &mut Engram, kind: ReinforceKind);

    /// Compute the effective weight given the balance
    /// (Scorer reads this, not `decay`).
    fn effective_weight(&self, engram: &Engram) -> f64;
}

pub enum ReinforceKind {
    Cited,        // another engram has this as a parent
    Retrieved,    // Substrate.get returned it as part of a query
    Gated,        // a gate compared against it and it held
    Surprised,    // a prediction error raised its informational value
    AgentQuoted,  // an agent read it and referenced it in a response
}
```

## 3. The rate law

The core tick:

```text
balance(t+Δt) = balance(t) - r * Δt - β * balance(t) * Δt
```

- First term is flat tax (Gesell's original): constant drain.
- Second term is exponential decay: keeps the value bounded below.

And on reinforcement:

```text
balance ← balance + bonus(kind) * novelty(engram)
```

Where `novelty` is `1 - max(similarity)` against the top-K HDC
neighbors (see `11-hyperdimensional-substrate.md`). **Novelty-weighted
reinforcement** is the key: citing a common engram gives it a tiny
bump, citing a rare engram gives it a big bump. This is the core
anti-hoarding mechanism — high-balance memory has to be *earning* its
balance from uniquely useful contributions.

## 4. What this enables that pure decay can't

### 4.1 Playbook freshness without manual GC

A playbook is an Engram. It earns balance every time an agent
successfully applies it, loses balance every tick it sits unused. When
the codebase drifts and the playbook stops working, its successful-use
rate drops; demurrage eats its balance; the Router stops proposing it.
**No human needs to "prune playbooks."**

### 4.2 Surprise-weighted retention

`ReinforceKind::Surprised` lets the Bus upweight Engrams whose
predictions were violated in interesting ways (see
`10-self-learning-cybernetic-loops.md`). This keeps the
high-information-content memories preferentially. It is Shannon's
surprise, operationalized as an economic bonus.

### 4.3 A natural "forgetting floor"

Balance can reach zero. At that point the Engram becomes a candidate
for cold storage or deletion — but its *hash* remains valid (lineage
doesn't break), the body just moves to a slower tier. This is the
primitive that lets us build a biologically plausible
short-term/long-term split *without hardcoded tiers*.

### 4.4 Composability with HDC consensus

Since `effective_weight` is a single float, HDC consensus bundles can
use it as a confidence coefficient:

```rust
consensus = Σ_i (fingerprint(e_i) * effective_weight(e_i))
```

Worldviews held by still-earning Engrams dominate; petrified ones
recede naturally.

## 5. Demurrage for the Policy layer

The same framework extends to Policy parameters themselves. Every
learned parameter (Scorer weight, Gate threshold, Router arm value)
can carry a balance. If a threshold hasn't been challenged by any
Pulse in a long time, its *confidence* should decay — not its value,
but the Policy's willingness to defend it against new evidence.

```rust
pub struct LearnedParam<T> {
    pub value: T,
    pub confidence: f64,     // demurrage-taxed
    pub last_challenge: Timestamp,
}
```

This unlocks **graceful relearning** — a long-stable parameter
eventually weakens enough that a modest amount of new evidence is
sufficient to move it. No explicit "reset" ever needed.

## 6. Configuration surface

```toml
[demurrage]
# Base rates
flat_tax_per_day         = 0.01    # r
exp_decay_per_day        = 0.005   # β
min_balance              = 0.0     # below this → cold tier

# Reinforcement bonuses
cited_bonus              = 0.05
retrieved_bonus          = 0.02
gated_bonus              = 0.03
surprised_bonus          = 0.15    # novelty-heavy
agent_quoted_bonus       = 0.08

# Policy-side demurrage
policy_confidence_tax    = 0.002
```

All of these can themselves be *learned* over time (demurrage-rates
that produce better retrieval quality reinforce themselves, via
prompt-experiment-style A/B measurement). The system tunes its own
forgetting rate.

## 7. Cold-tier graduation

Engrams whose balance hits the floor are graduated to a *cold*
substrate: same content-address, but the body moves off the hot path.
Retrieval becomes slower but still possible. This is the inverse of
the `graduate_to_engram` operation from `08-code-sketches.md` — we
already have the pattern of moving between fabrics; demurrage adds a
rule for *when*.

```rust
pub trait ColdSubstrate: Substrate {
    /// Freeze an engram into cold storage.
    fn freeze(&self, hash: EngramHash) -> Result<()>;

    /// Rehydrate on demand. Resets balance to a starting value.
    fn thaw(&self, hash: EngramHash) -> Result<Engram>;
}
```

A thaw is itself an event on the Bus, so interested operators can
update their caches.

## 8. Why this is a competitive moat

Most agent systems have two memory failure modes: *infinite growth*
(everything retained, retrieval quality collapses) and *brutal LRU*
(hard caps, biologically implausible, loses important rare events).

Demurrage gives us a third regime: **economically stable memory**.
Useful things stay warm, unused things fade gracefully, the floor is
adaptive, the rates are learnable. This property compounds with HDC
similarity and active-inference reinforcement — other frameworks can
replicate any one of these, but the *interaction* between them is
specific to Roko's substrate choices. The moat isn't any single
feature; it's the fact that Substrate + Bus + HDC + Demurrage +
Active-Inference stack into one coherent memory system and pull in
the same direction.

## 9. Observability

Three new metrics that surface the attention economy:

- **Balance histogram** per tier — shape of the distribution tells
  you whether the rates are too aggressive (everything is cold) or
  too lenient (hoarding).
- **Thaw rate** — how often cold engrams are pulled back. High thaw
  rate = the demurrage curve is too steep.
- **Reinforcement-by-kind** breakdown — what *kind* of use is
  keeping memory alive? If `Surprised` is low, the system isn't
  learning from prediction errors; if `Cited` is low, lineage is
  shallow.

These become first-class tiles on the `roko dashboard`.

## 10. Migration path

1. Add `balance`, `demurrage_paid`, `last_touched_at` fields.
   Backfill existing engrams with `balance = 1.0`.
2. Implement `Demurrage` trait and wire charge-on-read into Substrate.
3. Wire reinforcement into the five call sites (Router, Gate, Scorer,
   Composer, agent turns).
4. Have the Scorer read `effective_weight` instead of `decay`.
5. Deprecate `decay` field after one release cycle.

None of this breaks existing episode/playbook consumers — it just
makes their outputs self-trimming.

## 11. Open questions

- **Demurrage on Pulses?** A Pulse on the Bus is ephemeral by
  construction; does it need a tax? Probably not, but if Pulses can
  carry forward subscriptions, *retained-Pulses* might.
- **Taxation fairness across tiers** — should high-tier knowledge
  (distilled playbooks) tax at a different rate than raw episodes?
  Probably yes; distilled knowledge should be stickier to reflect
  the work that went into it.
- **Interaction with chain witnesses** (Phase 2) — a chain-witnessed
  Engram probably cannot be deleted even if balance hits zero; cold
  tier but never forget. Demurrage rate respects witness class.

## 12. Worked example: a playbook's life

Concrete scenario to show how the rate law, reinforcement, and tier
progression interact. Suppose `flat_tax_per_day = 0.01`,
`exp_decay_per_day = 0.005`, all defaults from §6.

Day 0. Agent distills a playbook. Balance = 1.0.

Days 1–7. Playbook is applied twice (cited) and matches gate
preconditions four times (retrieved). Tax: 7 × (0.01 + 0.005 × avg)
≈ 0.09. Reinforcement: 2 × 0.05 + 4 × 0.02 = 0.18. Net: balance rises
to ~1.09.

Days 8–30. Codebase drifts; preconditions stop matching. Zero
reinforcement. Tax: 23 × (0.01 + 0.005 × avg) ≈ 0.27. Balance drops
to ~0.82.

Day 31. An agent encounters a similar situation — HDC neighbor of
the playbook's fingerprint. The retrieval fires a `Retrieved` bonus
(0.02) but the playbook's advice fails its gate. The Policy marks
it `Surprised` (novelty-weighted 0.15 × novelty_score ≈ 0.08 effective).
Net change: +0.10. The Calibrator (§3 of 14) logs the failure.

Days 32–90. The playbook's pattern now fails consistently; each
failure is small novelty (cluster is known). Tax continues. Balance
drops below `min_balance = 0.0`. Substrate schedules `freeze(hash)`.

Day 91. Playbook moves to cold tier. Its hash is still resolvable;
lineage from Engrams that cited it still works. Full body is on slower
storage.

Day 400. A fork of the codebase reuses an old pattern. A new Engram
references the frozen playbook. Substrate `thaw(hash)` returns it
with balance reset to a starter value (configurable — default 0.3,
low enough that one failure sends it back to cold, high enough to
compete with fresh Engrams). The system is neither forgetting nor
hoarding; it is adapting with memory.

## 13. Interaction with the Composer and Scorer

Two concrete effects on the other operators:

- **Scorer**: reads `effective_weight(engram)` instead of the old
  `decay` field. A low-balance Engram scores lower across every
  axis, not just freshness. High-balance Engrams with strong axis
  scores dominate Router selection naturally. See
  `04-operators-generalized.md` §2 for the new Scorer signature.
- **Composer**: budget-aware composition now respects *attention
  budget*, not just token budget. Engrams with balance < 0.3 are
  candidates for the budget's last slots; they contribute only if
  nothing higher-balance is available. The composed prompt becomes
  preferentially fresh while retaining access to deep knowledge when
  fresh context isn't enough.

## 14. Operator knobs: when to tune which rate

The rate table from §6 has six tunable parameters. Operators
should touch them in this order:

1. **`min_balance`** — raise from 0.0 to, say, 0.1 if cold storage
   is growing too fast; lower if memory is bloated.
2. **`flat_tax_per_day`** — raise if memory is hoarding; lower if
   warming up new Engrams is too expensive.
3. **`surprised_bonus`** — raise this first when the system isn't
   learning from prediction errors visibly. `Surprised` is the
   novelty channel; cranking it emphasizes what's changing.
4. **`cited_bonus`** and **`retrieved_bonus`** — usually fine at
   defaults. Tune if the citation graph looks sparse or over-dense.
5. **`agent_quoted_bonus`** — raise in collaborative workflows where
   cross-agent citations matter (see c-factor §3.3).
6. **`exp_decay_per_day`** — touch rarely. Controls the asymptotic
   shape rather than the day-to-day gradient.

A self-tuning `DemurrageConfigPolicy` can subscribe to retrieval-
quality metrics and update these rates automatically (the rates
themselves are demurrage-taxed, per §5, so tuning that doesn't help
decays).

## 15. Dashboard for the attention economy

Four visible surfaces on `roko dashboard` (F-tab to be assigned,
expose via StateHub `demurrage_health` projection — see
`26-statehub-rearchitecture.md`):

- **Balance distribution**: stacked histogram of balance ranges,
  colored by Kind.
- **Reinforcement breakdown**: pie chart of `ReinforceKind` over the
  last 24h.
- **Thaw rate**: line graph of cold-to-warm thaws per hour.
- **Attention-leaderboard**: top 20 Engrams by balance, link to
  inspect.

Each tile answers a specific operator question ("is memory bloating?"
"what's keeping memory alive?" "are we forgetting too fast?" "what
does the system think is most important?"). Without these, demurrage
is invisible and untuneable.

--- END 12-knowledge-demurrage.md ---

# Batch REF12 — Knowledge demurrage supersedes decay field across decay chapter

**Refinement source**: `tmp/refinements/12-knowledge-demurrage.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- `docs/00-architecture/04-decay-variants.md` — demurrage supersedes decay; balance/reinforcement/thaw introduced.
- `docs/00-architecture/18-decay-tier-matrix.md` — tier graduation rules updated.
- `docs/00-architecture/25-attention-as-currency.md` — demurrage economic framing.
- `docs/00-architecture/20-configuration-schema.md` — demurrage rate keys documented.
- `docs/06-neuro/` and `docs/05-learning/` — playbook/episode retention updated.
- `docs/14-identity-economy/` — attention-economy references.

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/12-knowledge-demurrage.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.


## Required vocabulary (verify)

The verify step greps for: `demurrage|balance`

At least one changed file must mention it (case-insensitive) before the
batch is considered complete.

## Cross-references

Dependents — downstream refinements assuming this one has propagated
into docs: REF14, REF15, REF19, REF31

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
- Commit ready with message `refinements(REF12): Knowledge demurrage supersedes decay field across decay chapter`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
