# Refinements Batch REF13

Run id: run-20260416-221511
Attempt: 1
Model: gpt-5.4
Reasoning: high
Refinement source: /Users/will/dev/nunchi/roko/roko/tmp/refinements/13-collective-intelligence-c-factor.md
Target docs (candidates): docs/00-architecture/14-c-factor-collective-intelligence.md docs/13-coordination/11-collective-intelligence-metrics.md docs/13-coordination/ docs/00-architecture/INDEX.md

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

Suggested parallel split for batch `REF13`:

- worker: rewrite `docs/00-architecture/14-c-factor-collective-intelligence.md`
  to describe continuous measurement + Policy actuation.
- worker: update `docs/13-coordination/11-collective-intelligence-metrics.md`
  with the five-axis CohortMetrics and CohortWeightsLearner.
- worker: update `docs/13-coordination/INDEX.md` to link the c-factor chapter.

## Canonical refinement source

This is the verbatim refinement proposal that this batch must propagate
into `docs/`. Treat it as the authoritative source. Do not edit this
file; only edit docs under `docs/`.

--- BEGIN 13-collective-intelligence-c-factor.md ---

# Collective Intelligence & the c-factor

> **TL;DR**: Woolley et al. (2010, *Science*) showed that group
> performance across diverse tasks loads onto a single factor — "c"
> — analogous to the g-factor for individuals. c is predicted by
> social perceptiveness, turn-taking equality, and (notably) not by
> mean IQ of members. Roko's multi-agent runtime is a laboratory for
> measuring, optimizing, and mechanizing c. This doc proposes a
> concrete operationalization: the *Bus* is the conversation floor,
> Engrams are the artifacts, Pulses are the turns, and c-factor
> becomes a metric computed from their statistics. Improving it
> becomes an objective the Policy layer can optimize directly.

> **For first-time readers**: The c-factor is to groups what the g-factor
> is to individuals: one number that correlates with performance across
> a wide variety of tasks. Woolley 2010 found that mean IQ of group
> members didn't predict c; turn-taking equality and social
> perceptiveness did. Roko's runtime has observable analogs of all those
> process variables (Pulse authorship entropy, peer-prediction accuracy,
> citation reciprocity, delivery rate, HDC cloud diversity). This doc
> wires them into one metric, exposes it, and lets Policy optimize it.
> Read 10 (self-learning) and 11 (HDC) first; they provide the
> instrumentation this doc consumes.

## 1. The Woolley result and why it matters here

Three findings from the original paper and follow-ups:

1. **c exists**: across a battery of tasks, 40%+ of variance in group
   performance loads onto one factor.
2. **c is not mean IQ**: correlation between c and average member IQ
   is weak (r ≈ 0.15).
3. **c is driven by process**: turn-taking equality, social
   perceptiveness (measured via "Reading the Mind in the Eyes"),
   and proportion of women (partially mediated by social
   perceptiveness) all correlate strongly.

The mechanistic reading: **groups are intelligent to the extent that
information flows with low loss and reasonable equality between
members**. Bottlenecks (one dominant voice), silos (no turn-taking),
or poor perspective-taking (low social perceptiveness) all crush c.

Roko has all three failure modes available to it right now. A
dispatcher that always picks the fastest agent concentrates
turn-taking; a Router with no cross-lineage visibility creates silos;
agents that can't read each other's episodes lack social
perceptiveness. **We can do better because our "social" layer is
observable.**

## 2. Operationalizing c for Roko

### 2.1 The unit of observation

Define a *cohort*: a set of agents working on a related task (same
plan, same PRD, same parent episode). Over a time window, measure
their group output quality against an objective criterion (gates
passed, tests green, PRD reviewer score).

### 2.2 The process variables to measure

From the Woolley framework, adapted for agents:

| Human variable | Agent analog | How we measure |
|---|---|---|
| Turn-taking equality | Pulse authorship entropy per topic | `shannon(distinct_senders)` |
| Social perceptiveness | Ability to predict other agents' outputs | active-inference error on `peer.prediction` |
| Trust calibration | How often one agent cites another | citation-graph statistics |
| Channel openness | % of Pulses that reach all intended subscribers | delivery confirmation on the Bus |
| Cognitive diversity | HDC distance between agents' episode clouds | pairwise fingerprint distances |

All five are computable from the Bus + Substrate. **No new data
collection needed — just a stats layer.**

### 2.3 The c-score

Combine the five into a scalar via a small learned regression:

```rust
pub struct CohortMetrics {
    pub turn_taking_entropy: f64,     // [0, log2(n_agents)]
    pub peer_prediction_accuracy: f64, // [0, 1]
    pub citation_reciprocity: f64,     // [0, 1]
    pub delivery_rate: f64,            // [0, 1]
    pub hdc_diversity: f64,            // [0, 1]
}

pub fn c_factor(m: &CohortMetrics, w: &CohortWeights) -> f64 {
    w.a * m.turn_taking_entropy +
    w.b * m.peer_prediction_accuracy +
    w.c * m.citation_reciprocity +
    w.d * m.delivery_rate +
    w.e * m.hdc_diversity
}
```

Weights are fit by regressing `c_factor` against realized cohort
outcomes (gate-pass rate, task success). The regression itself runs as
an active-inference loop — see `10-self-learning-cybernetic-loops.md`.

## 3. Improving c

Once c is measured, the Policy layer can *optimize* it. Five levers:

### 3.1 Turn-taking

If entropy is low, the dispatcher's Router is too greedy. Add a
*temperature* that softens top-1 selection in proportion to recent
entropy deficit. Gesell's demurrage (see `12`) applied to *agent
balance* — an agent that has spoken recently pays a tax on its next
bid.

### 3.2 Social perceptiveness

Each agent publishes `peer.prediction` Pulses: "I think agent X would
say Y." Reality arrives on `peer.outcome`. Prediction error feeds the
agent's own learning. Agents that model each other well get routed to
collaborative tasks; those that can't get routed to parallelizable
(non-collaborative) ones. This is **cognitive specialization by
measured empathy.**

### 3.3 Trust calibration

Citation reciprocity and citation quality. An agent that cites another
agent's Engram which later fails a gate loses trust-credit. Accrues
trust-credit for citations that pass. Trust is per-directed-pair and
per-topic (agent A might trust B on Rust syntax, not on
database-schema design).

### 3.4 Channel openness

The Bus already tracks delivery; expose metrics. Subscribers that drop
messages due to backpressure, circuit breakers, or auth failures
*reduce c directly*. Ops dashboards should plot c alongside CPU; they
move together.

### 3.5 Cognitive diversity

Use HDC cloud distance. If all agents' episode fingerprints collapse
to the same region, diversity is low — and diversity is part of c.
Policy can inject diversity pressure: spawn agents with deliberately
different system prompts, tool sets, or model families when cloud
distance drops below threshold. **c-factor as a regularizer against
cognitive monoculture.**

## 4. The Surowiecki conditions as gates

James Surowiecki's *Wisdom of Crowds* (2004) lists four conditions:
diversity of opinion, independence, decentralization, aggregation.
These become explicit gates in Roko's pipeline:

```rust
pub struct WisdomGate {
    min_hdc_diversity: f64,       // diversity of opinion
    max_lineage_overlap: f64,     // independence
    max_sender_share: f64,        // decentralization
    aggregator: Box<dyn Aggregator>, // aggregation method
}
```

Before a consensus Engram is finalized, its inputs must pass the
WisdomGate. If 80% of inputs share a lineage ancestor, that's not a
wisdom-of-crowds consensus — it's an echo chamber. **We can detect
and refuse echo chambers structurally.**

## 5. Aggregation methods as first-class

The fourth Surowiecki condition — aggregation — is where most systems
cheat by averaging. HDC gives us four genuinely different options:

1. **Bundle (majority vote)**: XOR-add fingerprints, binarize. Classic
   wisdom-of-crowds.
2. **Bind (structured)**: tag each agent's contribution with their
   identity fingerprint, bind, then bundle. Preserves *who said what*
   while still collapsing.
3. **Weighted bundle**: each fingerprint multiplied by agent's trust
   score for the topic. Bayesian flavor.
4. **Cleanup to codebook**: bundle, then snap to nearest known
   Engram. Forces output to be *expressible* in existing vocabulary.

Each has different properties under different team compositions.
Policy picks. Aggregation is no longer a hardcoded `mean()` but an
operator chosen based on c-factor stats.

## 6. Anti-groupthink primitives

A system optimizing for c can overshoot into groupthink if not
careful. Three countermeasures:

### 6.1 Devil's-advocate role

A canonical role-prompt whose job is to generate a *maximally-opposed*
Pulse on every consensus topic. Policy spawns one when HDC diversity
drops below threshold. This is the "red team" made structural.

### 6.2 Outsider-injection

Periodically, Policy routes a task to an agent with *zero lineage
overlap* with the active cohort. Its output is published but
labeled: downstream consumers know this is a deliberate outsider
perspective and weight accordingly.

### 6.3 Minority report preservation

Demurrage on *dissenting* Engrams is softer than on consenting
ones — we explicitly subsidize minority positions for longer. The
Bus carries a `consensus_distance` tag; high-distance Engrams get a
demurrage discount. This prevents the majority from simply starving
minority views of attention-credit.

## 7. c-factor as a dashboard tile

```
┌─ Cohort Intelligence (last 24h) ──────────────────────┐
│ c-factor: 0.72 (↑ 0.08 from last window)              │
│ turn-taking entropy:      2.31 / 3.00 (7 agents)      │
│ peer prediction accuracy: 61%                         │
│ citation reciprocity:     0.54                        │
│ delivery rate:            99.1%                       │
│ HDC diversity:            0.68                        │
│                                                       │
│ Weakest link: peer prediction (consider: rotate pairs)│
│ Groupthink risk:  LOW (min_pair_distance = 0.41)      │
└───────────────────────────────────────────────────────┘
```

This becomes a first-class tab on `roko dashboard`, and an API route
on `roko serve`.

## 8. Why most multi-agent frameworks can't measure this

Frameworks like LangGraph, AutoGen, CrewAI have agents, and some have
shared state. None have:

1. A **content-addressed substrate with lineage** — needed for
   citation-reciprocity and lineage-overlap metrics.
2. A **first-class Bus with delivery confirmation** — needed for
   channel-openness and turn-taking metrics.
3. An **HDC fingerprint on every artifact** — needed for diversity.
4. A **demurrage-driven attention economy** — needed to prevent
   echo-chamber drift without manual intervention.

Roko has or can easily add all four. c-factor measurement *falls out*
of the architecture rather than being bolted on. This is the
genuine moat: measuring collective intelligence is trivial when your
substrate is already designed for it.

## 9. Cross-cohort c: coalitions of coalitions

Once c is measured per cohort, the same math applies *between*
cohorts. A team-of-teams c-factor. This is the primitive for Phase
2+ chain architecture: chains of agents coordinating via
witness-signed Pulses can have their inter-cohort c measured and
optimized exactly the same way.

Cohorts with low c get merged (break silos). Cohorts with too-high c
and low diversity get split (break monocultures). The org-chart of
agent teams becomes self-tuning.

## 10. Implementation phases

1. **Metrics-only**: compute CohortMetrics from existing Bus and
   Substrate data. Log to `.roko/learn/c-factor.jsonl`. No behavior
   change. Two days of work.
2. **Dashboard tile + alerts**: expose in TUI and HTTP. One day.
3. **Passive optimization**: CohortWeights fit via active-inference;
   Policy reports c as a signal but doesn't act on it. One week.
4. **Active optimization**: Policy acts on c — temperature bumps,
   devil's-advocate spawning, outsider injection. Two weeks.
5. **Cross-cohort c and auto-org**: Phase 2. Open-ended.

Steps 1–2 are risk-free wins that surface information the team
already has. Steps 3+ get structurally interesting.

## 11. The net-new claim

There is published work measuring c in human teams, and there is
published work on multi-agent coordination. There is (to our
knowledge) no system that *measures c continuously in a running
agent runtime and closes the loop on it*. This combination is
specific to Roko's architecture, publishable, and a genuine
differentiator.

## 12. Fitting CohortWeights against outcomes

The regression from §2.3 isn't abstract; it's a small online learner
that subscribes to the Bus. Pseudo-Rust:

```rust
pub struct CohortWeights {
    pub a: f64,   // turn-taking entropy
    pub b: f64,   // peer prediction accuracy
    pub c: f64,   // citation reciprocity
    pub d: f64,   // delivery rate
    pub e: f64,   // HDC diversity
    pub bias: f64,
}

/// Policy that subscribes to cohort-completion Pulses and fits the
/// CohortWeights via online stochastic gradient on squared error vs
/// the observed outcome (gate-pass rate).
pub struct CohortWeightsLearner<B: Bus> {
    pub bus: Arc<B>,
    pub weights: parking_lot::RwLock<CohortWeights>,
    pub learning_rate: f64,         // e.g. 1e-3
}

impl<B: Bus> CohortWeightsLearner<B> {
    pub async fn run(self: Arc<Self>) {
        let filter = TopicFilter::Exact(Topic::new("cohort.completed"));
        let mut rx = self.bus.subscribe(filter).await.unwrap();
        while let Some(pulse) = rx.recv().await {
            let Some(obs) = parse_observation(&pulse) else { continue };
            let prediction = predict(&*self.weights.read(), &obs.metrics);
            let err = obs.outcome - prediction;
            let mut w = self.weights.write();
            w.a += self.learning_rate * err * obs.metrics.turn_taking_entropy;
            w.b += self.learning_rate * err * obs.metrics.peer_prediction_accuracy;
            w.c += self.learning_rate * err * obs.metrics.citation_reciprocity;
            w.d += self.learning_rate * err * obs.metrics.delivery_rate;
            w.e += self.learning_rate * err * obs.metrics.hdc_diversity;
            w.bias += self.learning_rate * err;
            drop(w);
            let _ = self.bus.publish(emit_weights_update(&*self.weights.read())).await;
        }
    }
}
```

The weights drift toward whatever configuration best explains observed
outcomes. Teams with different task distributions naturally end up
with different weights — Woolley's c isn't one number for everyone,
it's a shape of the regression learned in context. Confidence intervals
on the weights themselves are maintained via the same active-inference
machinery from `10-self-learning-cybernetic-loops.md`.

## 13. c-factor is a covariate, not an objective

Critical caveat (also in `15-exponential-scaling.md` §4.2): **the
Policy layer should not optimize c directly**. Optimizing c can be
trivially gamed by routing all work to easy tasks. The correct use:

1. **c is a measurement.** Publish it. Inspect it. Correlate it with
   outcomes.
2. **c is a diagnostic.** A drop in c alongside a drop in outcomes
   is a signal to intervene. A drop in c alongside stable outcomes is
   noise.
3. **c-optimization levers are applied conditionally.** Turn-taking
   temperature, devil's advocate, outsider injection — these fire
   when c is low *and* outcomes are suffering. Never when only c is
   low.

The loop is: observe c and outcome, compute correlation, intervene
on *process* variables when correlation predicts intervention will
help, measure again. This is standard regulatory control, not
reward hacking.

## 14. Cross-synergies

- **Demurrage (12)** §5 — agents accruing balance too fast
  (dominant voices) get agent-level demurrage that taxes their bid
  in the next turn. This directly lowers `max_sender_share` in the
  WisdomGate.
- **Heuristics (14)** §7 — peer-heuristic models are the
  operationalization of `peer_prediction_accuracy`. An agent that
  models its teammates' priors well scores high on that metric.
- **Worldview (14)** §5 — multiple active worldviews is the
  structural answer to the `hdc_diversity` axis.
- **Replication ledger (16)** — every agreed-upon claim gets
  measured twice, once by the cohort's aggregate and once by any
  individual. High cross-cohort c correlates with stable
  replications.
- **UX (23 §10, 30 §2.8)** — the c-factor and its components
  surface in the TUI and web UI as a tile. Operators can *see*
  what the group is doing well or poorly.
- **Deployment UX (24)** §5 — `roko.c_factor` is a first-class
  Prometheus metric; alerts on it fire like any other SLI.

--- END 13-collective-intelligence-c-factor.md ---

# Batch REF13 — c-factor continuous measurement across coordination + architecture

**Refinement source**: `tmp/refinements/13-collective-intelligence-c-factor.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- `docs/00-architecture/14-c-factor-collective-intelligence.md` — rewrite for continuous measurement + Policy actuation.
- `docs/13-coordination/11-collective-intelligence-metrics.md` — five-axis CohortMetrics, CohortWeightsLearner.
- `docs/13-coordination/INDEX.md` — link the c-factor chapter.
- `docs/00-architecture/INDEX.md` — c-factor tile referenced.

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/13-collective-intelligence-c-factor.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.


## Required vocabulary (verify)

The verify step greps for: `c.?factor`

At least one changed file must mention it (case-insensitive) before the
batch is considered complete.

## Cross-references

Dependents — downstream refinements assuming this one has propagated
into docs: REF19, REF31

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
- Commit ready with message `refinements(REF13): c-factor continuous measurement across coordination + architecture`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
