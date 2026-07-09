# Refinements Batch REF33

Run id: run-20260416-221511
Attempt: 1
Model: gpt-5.4
Reasoning: high
Refinement source: /Users/will/dev/nunchi/roko/roko/tmp/refinements/33-observability-telemetry.md
Target docs (candidates): docs/19-deployment/ docs/00-architecture/21-performance-numerical-stability.md docs/00-architecture/32-comprehensive-test-strategy.md

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

Suggested parallel split for batch `REF33`:

- worker: add/update files under `docs/19-deployment/` with the logs /
  metrics / traces / events / replay surfaces and the Roko-specific metrics.
- worker: update `docs/00-architecture/21-performance-numerical-stability.md`
  where performance observability is discussed.
- worker: update `docs/00-architecture/32-comprehensive-test-strategy.md`
  with replay-as-test framing.

## Canonical refinement source

This is the verbatim refinement proposal that this batch must propagate
into `docs/`. Treat it as the authoritative source. Do not edit this
file; only edit docs under `docs/`.

--- BEGIN 33-observability-telemetry.md ---

# Observability & Telemetry

> **TL;DR**: The most distinctive claim Roko makes — every agent
> turn, every gate, every heuristic, every cost in one
> content-addressed, queryable substrate — lives or dies by its
> observability. This doc consolidates the metrics, logs, traces,
> events, replay primitives, and cost visibility scattered across
> 22, 23, 24, 26, 27, 30, 32 into a single instrumentation spec.
> What ships with the binary, what's pluggable, what's Roko-specific
> (c-factor, demurrage balance, calibration drift), and what
> integrates with existing monitoring stacks.

> **For first-time readers**: "Observability" here covers five
> things: structured logs, Prometheus-compatible metrics, OpenTelemetry
> traces, Bus events (internally), and episode replay (time-travel).
> Read 24 §5 first for the deployment-level overview; this doc is
> the depth. The key distinction: *generic* observability (every
> framework has metrics) vs *Roko-specific* (c-factor, demurrage
> balance, heuristic calibration drift). The Roko-specific metrics
> are the reason the system is worth running; they deserve first-class
> treatment.

## 1. The four telemetry surfaces

Roko produces telemetry in four modes, each with a different consumer:

1. **Logs** (stderr, JSON by default) — for humans inspecting a
   session, for `docker logs`/`journalctl`/log aggregators.
2. **Metrics** (`/metrics` Prometheus exposition) — for scraping by
   Prometheus, VictoriaMetrics, Datadog, etc.
3. **Traces** (OpenTelemetry spans over OTLP) — for distributed
   tracing: which operator took how long, how did the call tree
   branch, which agent owns which span.
4. **Events** (Bus pulses + StateHub projections) — internal-first,
   but externally consumable via `27-realtime-event-surface.md`.

Each is pluggable; defaults ship. Operators can redirect any of the
four without changing kernel code.

## 2. Structured log format

Every log line is a single JSON object:

```json
{
  "ts": "2026-04-16T13:42:15.312Z",
  "level": "info",
  "target": "roko_orchestrator::plan",
  "fields": {
    "plan_id": "p_abc123",
    "task_id": "t_def456",
    "agent_id": "ag_gh78",
    "message": "task dispatched",
    "elapsed_ms": 12
  },
  "span": {
    "name": "op.route",
    "id": "sp_1a2b3c",
    "parent_id": "sp_0a0b0c"
  },
  "trace_id": "4a1e9b..."
}
```

Every Bus publish emits a corresponding log line if log level is
`info` or below. Large bodies (>1 KB) are replaced with `{hash: ...,
len: N}` to keep logs parseable.

Plain-text mode (`--log-format human`) for interactive sessions;
default is JSON.

## 3. Generic metrics (every runtime has these)

| Metric | Type | Labels | Purpose |
|---|---|---|---|
| `roko.http.requests_total` | counter | method, path, status | HTTP control plane traffic |
| `roko.http.request_duration_seconds` | histogram | method, path | Latency |
| `roko.process.cpu_seconds_total` | counter | — | CPU usage |
| `roko.process.memory_bytes` | gauge | — | RSS |
| `roko.process.open_files` | gauge | — | fd count |
| `roko.tokio.tasks_active` | gauge | — | async task count |
| `roko.tokio.blocking_tasks` | gauge | — | blocking pool usage |

These are table stakes; standard exporters handle them.

## 4. Safety-relevant metrics (from 32)

| Metric | Type | Labels | Purpose |
|---|---|---|---|
| `roko.safety.authz_total` | counter | role, action, decision | Who's getting denied |
| `roko.safety.confirms_pending` | gauge | role | Unanswered confirms |
| `roko.safety.escalations_total` | counter | reason | Escalation rate |
| `roko.safety.taint_propagations` | counter | from, to | Taint fan-out |
| `roko.safety.plugin_violations_total` | counter | plugin, kind | Sandbox breaches |
| `roko.network.egress_total` | counter | host, status | External calls |

An operator dashboard for security builds entirely on these.

## 5. Roko-specific metrics (the interesting ones)

Metrics that *only* make sense in Roko. These are the ones to
surface loudly:

### 5.1 Collective intelligence

| Metric | Type | Labels | From |
|---|---|---|---|
| `roko.c_factor` | gauge | cohort | 13 §2.3 |
| `roko.turn_taking_entropy` | gauge | cohort | 13 §2.2 |
| `roko.peer_prediction_accuracy` | gauge | cohort | 13 §2.2 |
| `roko.citation_reciprocity` | gauge | cohort | 13 §2.2 |
| `roko.hdc_diversity` | gauge | cohort | 13 §2.2 |
| `roko.cohort_delivery_rate` | gauge | cohort | 13 §2.2 |

### 5.2 Memory economy

| Metric | Type | Labels | From |
|---|---|---|---|
| `roko.demurrage.balance_p50` | histogram | kind | 12 §9 |
| `roko.demurrage.balance_p95` | histogram | kind | 12 §9 |
| `roko.demurrage.thaw_total` | counter | kind | 12 §9 |
| `roko.demurrage.reinforce_total` | counter | kind, reinforce_kind | 12 §9 |
| `roko.substrate.engrams_warm` | gauge | kind | 12 |
| `roko.substrate.engrams_cold` | gauge | kind | 12 |
| `roko.substrate.query_latency_ms` | histogram | query_kind | existing |
| `roko.substrate.query_similar_latency_ms` | histogram | — | 11 §4.1 |

### 5.3 Learning

| Metric | Type | Labels | From |
|---|---|---|---|
| `roko.heuristic.total` | gauge | calibration_bucket | 14 |
| `roko.heuristic.calibration_brier` | histogram | heuristic_id | 14 §2 |
| `roko.heuristic.trials_total` | counter | heuristic_id | 14 §3.3 |
| `roko.replication.ledger_total` | gauge | status | 16 §5 |
| `roko.prediction.ema_error` | gauge | operator | 10 §5 |
| `roko.prediction.rmse` | gauge | operator | 10 §10 |

### 5.4 Gate pipeline

| Metric | Type | Labels | From |
|---|---|---|---|
| `roko.gate.verdicts_total` | counter | gate, passed | existing |
| `roko.gate.failure_rate` | gauge | gate | 10 §7.1 |
| `roko.gate.latency_ms` | histogram | gate | existing |
| `roko.gate.pipeline_duration_ms` | histogram | — | existing |

### 5.5 Bus

| Metric | Type | Labels | From |
|---|---|---|---|
| `roko.bus.pulses_total` | counter | topic | 03 |
| `roko.bus.ring_occupancy` | gauge | bus_name | 03 §8 |
| `roko.bus.ring_capacity` | gauge | bus_name | 03 §2 |
| `roko.bus.subscribers_active` | gauge | topic_pattern | 03 |
| `roko.bus.lagging_subscribers_total` | counter | topic_pattern | 03 §8 |

### 5.6 Cost

| Metric | Type | Labels | From |
|---|---|---|---|
| `roko.cost.tokens_total` | counter | model, role | 24 §10 |
| `roko.cost.usd_total` | counter | model, role | 24 §10 |
| `roko.cost.budget_remaining_usd` | gauge | budget_scope | 24 §10 |
| `roko.cost.cascade_router_decisions_total` | counter | model_selected | existing |

All metrics follow Prometheus naming conventions (lowercase,
underscore-separated, `_total` suffix for counters).

## 6. Traces — OpenTelemetry spans

Every operator boundary emits a span:

```
op.sense        (step 1 of the loop)
  ├─ substrate.query
  └─ bus.receive
op.assess       (step 2)
  └─ router.select
     └─ cascade_router.decide
op.compose      (step 3)
  └─ composer.compose
     └─ substrate.query_similar      (HDC retrieval)
op.act          (step 4)
  └─ agent.llm_call
     ├─ tool.call               (multiple)
     └─ bus.publish             (token chunks)
op.verify       (step 5)
  └─ gate.pipeline
     ├─ gate.compile
     ├─ gate.test
     └─ gate.clippy
op.persist      (step 6a)
  └─ substrate.put
op.broadcast    (step 6b)
  └─ bus.publish
op.react        (step 7)
  └─ policy.decide            (per-policy child span)
```

Span attributes include `operator_id`, `principal_id`,
`content_hash`, `pulse_seq` where relevant. Trace id flows through
the ctx argument of every operator.

Exporters: OTLP, Jaeger, Zipkin. `OTEL_EXPORTER_OTLP_ENDPOINT` env
configures.

## 7. Events — StateHub projections as a telemetry target

Consumers that want *typed, filterable, queryable* telemetry subscribe
to projections via `26-statehub-rearchitecture.md`. This is a
first-party alternative to scraping Prometheus:

- `cohort_health` → live c-factor snapshot + roster.
- `gate_pipeline` → current rung status, pass/fail counts.
- `bus_stats` → pulses/sec by topic.
- `substrate_stats` → balance histogram, tier sizes.
- `cost_meter` → per-model spend.
- `safety_events` → recent authz denials, confirms, escalations.
- `replication_ledger` → claim status table.
- `calibration_curves` → per-operator error trends.

A Grafana data source plugin can consume any projection as a stream
or as a point-in-time snapshot. Operators get live, typed
observability without rolling their own scraping.

## 8. Alerts

Default alert rules shipping with Roko (Prometheus/Alertmanager
format):

```yaml
# alerts/roko.yml
groups:
- name: roko_critical
  rules:
    - alert: RokoCFactorDropping
      expr: rate(roko.c_factor[10m]) < -0.05
      for: 15m
    - alert: RokoGatePipelineStalled
      expr: rate(roko.gate.verdicts_total[5m]) == 0
      for: 10m
    - alert: RokoSafetyEscalationSurge
      expr: rate(roko.safety.escalations_total[5m]) > 1
    - alert: RokoDemurrageSubstrateBloat
      expr: roko.substrate.engrams_warm > 10_000_000
    - alert: RokoCalibrationDriftSpike
      expr: rate(roko.prediction.ema_error[30m]) > 0.1
    - alert: RokoBusRingSaturation
      expr: roko.bus.ring_occupancy / roko.bus.ring_capacity > 0.9
    - alert: RokoCostSurge
      expr: rate(roko.cost.usd_total[1h]) > 5
    - alert: RokoPluginViolationsSpike
      expr: rate(roko.safety.plugin_violations_total[5m]) > 0.1
```

Each alert has a runbook URL in its `annotations` pointing at
`docs/runbooks/<name>.md`.

## 9. Cost dashboard as a first-class page

Given Roko's cost visibility claim (24 §10), cost gets its own
telemetry surface beyond raw metrics:

- **Per-session spend**: live counter in the CLI prompt and web UI.
- **Per-task breakdown**: after a `plan run`, a table like
  `task_id | model | tokens_in | tokens_out | usd | seconds`.
- **Per-role historical**: `roko cost report --period 7d --by role`.
- **Per-model**: which models are earning their keep.
- **Budget vs burn**: visualizes budget consumption rate over time.

Tiles on the web UI Home (29 §3.1) and the TUI Cost tab read these
from the `cost_meter` projection.

## 10. Replay as observability

Time-travel through any decision:

```bash
# Replay an episode inspecting what the agent saw
roko replay ep_12345 --trace
# Replay with alternate config to test sensitivity
roko replay ep_12345 --override "demurrage.flat_tax=0.02"
# Replay to generate an audit report
roko replay ep_12345 --audit > report.md
```

Replay consumes the Engram + Pulse history of the episode (if the
ring hadn't wrapped on Pulses, a fresh subscribe — if it had,
reconstructed from graduated Engrams). Operators walking through a
postmortem use replay to answer "what did the agent know at this
moment?"

## 11. Self-observability of the observability surface

The observability layer itself is instrumented:

- Log producer queue depth.
- Log dropped lines (when queue overflows).
- Metric scrape duration.
- Trace span drops (when the exporter can't keep up).
- StateHub projection delta latency.

`/readyz` returns `not ready` if any observability sink is
unavailable beyond a threshold, so rollouts don't black-hole
observability.

## 12. Integration with existing stacks

Typical deployment stacks and how Roko plugs in:

| Stack | Roko surface used | Integration |
|---|---|---|
| Prometheus + Grafana | `/metrics`, alerts | `helm install prometheus-community/...`; Grafana dashboards shipped |
| Loki / Elastic / Datadog Logs | stdout JSON | Standard container log shipping |
| Jaeger / Zipkin / Tempo / Honeycomb | OTLP | `OTEL_EXPORTER_OTLP_ENDPOINT` env |
| Sentry / Bugsnag | stderr + crash handler | Crash reporter plugin (tier-3) |
| Slack / PagerDuty | Alert routes | Alertmanager receiver |
| Custom dashboards | StateHub + realtime | `@roko/client` subscription |

Roko provides stock Grafana dashboards in
`deployment/grafana/roko-overview.json`,
`roko-safety.json`, `roko-cognitive.json` (the last one is the
unique surface no other framework has).

## 13. Structured-log heuristics

For humans grepping logs:

- Every log line that crosses a safety boundary includes
  `safety_decision=<decision>`.
- Every cost-bearing action includes `usd=<amount>`.
- Every Engram write includes `engram_kind` and `engram_hash`.
- Every Pulse publish includes `topic`.
- Every gate verdict includes `gate`, `passed`.

Shared labels enable aggregations across log lines from different
subsystems. An operator grepping
`safety_decision=escalate` finds every escalation in a session.

## 14. Debug mode

`roko --debug` enables:

- All tracing spans logged as events (even ones the exporter
  discards).
- Every Bus publish also logged at `info`.
- Every Substrate put logged at `debug` with body length.
- Profiling data dumped to `.roko/debug/profile-<ts>.json`.
- `RUST_BACKTRACE=1` implicit.

`--debug` is for postmortems and development. Production always
runs without it.

## 15. Retention and sampling

Not all telemetry retains forever:

- Metrics: whatever the downstream stack retains (Prometheus default
  15 days).
- Logs: whatever the log shipping stack retains.
- Traces: sampling rate configurable; default 10% sample; 100% sample
  on error.
- Bus Pulses: ring-buffer retention (default 4096 per bus); graduated
  Engrams persist per demurrage.
- Engrams: demurrage-managed; see 12.
- Custody records: retained long-term (compliance-tier storage);
  optionally chain-witnessed (Phase 2+).

Each has a CLI for local inspection:
`roko logs tail`, `roko metrics show`, `roko traces find`,
`roko bus replay`, `roko substrate stats`, `roko custody list`.

## 16. The observability pitch

"Every agent turn has a trace. Every heuristic has a calibration
curve. Every gate has a latency histogram. Every cohort has a
c-factor you can watch in real time. Every cost is attributed.
Every safety decision is audited. Every decision is replayable."

Few frameworks make any of these claims. None make all of them. Roko
can, and should, because the substrate was designed for it. The
instrumentation in this doc is the line between that claim being
marketing and being reality.

## 17. Staging

Most of the generic surface already exists in `roko-runtime`. The
gap list:

1. **Roko-specific metrics** (§5) — two weeks to wire the dozens of
   new gauges/counters to the actual subsystems. Most of the data
   already exists on the Bus; this is plumbing.
2. **Default Grafana dashboards** — one week.
3. **StateHub projections for telemetry** — two weeks, depends on 26
   landing.
4. **Alert rules and runbooks** — one week.
5. **Cost report CLI** — three days.
6. **Replay-with-override CLI** — one week.
7. **Grafana data-source plugin for StateHub** — two weeks.

Total: two months of focused observability work. After, Roko is
legible.

## 18. Cross-references

- Bus-related metrics home: `03-bus-as-first-class.md` §5, §8.
- StateHub surface that feeds most live telemetry:
  `26-statehub-rearchitecture.md`.
- Realtime surface for external consumers:
  `27-realtime-event-surface.md`.
- Cost story: `24-deployment-ux.md` §10, `28-cli-parity-familiar-workflows.md` §9.
- Safety events: `32-safety-sandbox-provenance.md` §14.
- c-factor dashboard tile: `13-collective-intelligence-c-factor.md` §7.
- Demurrage balance histogram: `12-knowledge-demurrage.md` §9.
- Rich UX primitives that render telemetry:
  `30-rich-ux-primitives.md`.

--- END 33-observability-telemetry.md ---

# Batch REF33 — Observability + telemetry across deployment + architecture

**Refinement source**: `tmp/refinements/33-observability-telemetry.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- `docs/19-deployment/` — logs / metrics / traces / events / replay surfaces; Roko-specific metrics (c-factor, demurrage, calibration).
- `docs/00-architecture/21-performance-numerical-stability.md` — observability references.
- `docs/00-architecture/32-comprehensive-test-strategy.md` — replay-as-test framing.

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/33-observability-telemetry.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.


## Required vocabulary (verify)

The verify step greps for: `telemetry|observability|metric`

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
- Commit ready with message `refinements(REF33): Observability + telemetry across deployment + architecture`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
