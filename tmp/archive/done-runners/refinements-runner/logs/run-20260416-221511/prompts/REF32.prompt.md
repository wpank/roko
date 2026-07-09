# Refinements Batch REF32

Run id: run-20260416-221511
Attempt: 1
Model: gpt-5.4
Reasoning: high
Refinement source: /Users/will/dev/nunchi/roko/roko/tmp/refinements/32-safety-sandbox-provenance.md
Target docs (candidates): docs/11-safety/ docs/00-architecture/05-provenance-and-attestation.md docs/00-architecture/26-cognitive-immune-system.md

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

Suggested parallel split for batch `REF32`:

- worker: update `docs/11-safety/` with the safety spine: role auth, sandboxes,
  pre/post checks, taint, attestation, chain-of-custody.
- worker: update `docs/00-architecture/05-provenance-and-attestation.md`
  with the Custody record shape and the attestation levels.
- worker: update `docs/00-architecture/26-cognitive-immune-system.md` with
  the taint-propagation + detection framing.

## Canonical refinement source

This is the verbatim refinement proposal that this batch must propagate
into `docs/`. Treat it as the authoritative source. Do not edit this
file; only edit docs under `docs/`.

--- BEGIN 32-safety-sandbox-provenance.md ---

# Safety, Sandbox, and Provenance

> **TL;DR**: Safety in Roko isn't a single module — it's a spine that
> runs orthogonally across every layer. Role-based tool authorization,
> per-tier plugin sandboxes, pre/post check pairs, taint propagation,
> cryptographic attestation, chain-of-custody records, and a permission
> gradient for human-in-the-loop checkpoints all share one vocabulary
> and one point of enforcement. This doc pulls those threads out of 17,
> 23, 24, 25, and 26 into a single defensive story. The goal: an
> operator can answer "who did what, with what authorization, with
> what consequence?" for any action Roko took.

> **For first-time readers**: This is the doc for security reviewers,
> compliance officers, and anyone deploying Roko in a regulated context.
> It consolidates the safety material scattered across the earlier
> refinements into one defensive spine. Start with §2 (the permission
> model), §4 (human-in-the-loop), and §5 (chain of custody); the rest
> adds depth.

## 1. What "safety" means here

Three distinct concerns, all of which belong in this doc:

1. **Authorization**: who (user, agent, plugin) is allowed to do
   what. Tool calls, Engram writes, Pulse publishes, topic
   subscriptions, substrate reads. Enforcement is trait-level.
2. **Isolation**: when untrusted code runs (tier-3 tool, tier-4
   native extension, tier-5 WASM extension), it can't violate its
   declared capability envelope. Enforcement is layer-level
   (process, container, WASM sandbox).
3. **Provenance**: for every action that mattered, a durable record
   exists of who initiated it, what they were trying to do, what
   heuristics influenced them, what gates approved, and what
   resulted. Enforcement is Substrate-level with optional chain
   witnesses.

The spine stitches all three so operators configure them in one
place and the runtime enforces them consistently.

## 2. The permission model

Every permission-gated action names:

- **Principal**: user id, agent id, or plugin id.
- **Action**: a verb from a controlled vocabulary.
- **Target**: Engram kind, Bus topic, tool id, substrate region,
  file-system path, network endpoint.
- **Context**: the TypedContext (25 §8.1) at call time.
- **Authorization source**: role grant, session approval, one-shot
  approval, or plugin manifest.

Authorization is checked in `roko-agent/src/safety/` (extending
today's layer) using a deterministic function:

```rust
pub fn authorize(
    principal: &Principal,
    action: &Action,
    target: &Target,
    ctx: &TypedContext,
    env: &SafetyEnv,   // role grants, session approvals, manifest permissions
) -> AuthzDecision;

pub enum AuthzDecision {
    Allow,
    AllowWithConfirm { prompt: String },
    AllowOnce,
    Deny { reason: String },
    Escalate { to: EscalationTarget },
}
```

The decision never silently allows — `AllowWithConfirm` bubbles
to the UI; `Escalate` pages a human.

## 3. Default permission table

Each action has a default decision per principal role. Custom
profiles (25) override. The base table:

| Action | researcher | planner | implementer | reviewer | ops |
|---|---|---|---|---|---|
| Read file under workspace | Allow | Allow | Allow | Allow | Allow |
| Read file outside workspace | Deny | Deny | Deny | Deny | Confirm |
| Write file under workspace | Deny | Deny | Allow | Deny | Confirm |
| Write file outside workspace | Deny | Deny | Deny | Deny | Escalate |
| Run shell command | Deny | Deny | Confirm | Deny | Confirm |
| Make network request | AllowOnce | Deny | Deny | Deny | Confirm |
| Install a dependency | Deny | Deny | Confirm | Deny | Escalate |
| Delete a file | Deny | Deny | Escalate | Deny | Escalate |
| Git commit | Deny | Deny | Confirm | Deny | Deny |
| Git push | Deny | Deny | Escalate | Deny | Escalate |
| Call external API with user credential | Confirm | Deny | Deny | Deny | Confirm |
| Publish Pulse on safety.* topic | Deny | Deny | Deny | Deny | Allow |

Rows extend per-domain (blockchain profile adds "sign transaction,"
ops profile adds "modify kubernetes resource," etc.). Profile
bundles ship their own tables that merge with the base.

## 4. Human-in-the-loop checkpoints

Three categories of checkpoint, each with a UX expectation:

### 4.1 Permission checkpoint

Before an action whose default is `AllowWithConfirm` or
`AllowOnce`. Presented as a modal-style prompt (CLI: inline Y/n;
TUI: popup; Web: dialog with details).

```
Allow: install crate "serde_json 1.0.108"
  Principal: agent:implementer-01
  Source:    heuristic h.042 "When parsing JSON, prefer serde_json"
  Confirm:   [Y]es once  [A]llow this session  [N]o  [E]scalate
```

Remember within session: `AllowOnce` becomes `Allow` for the rest of
the session scoped to the same action+target.

### 4.2 Ambiguity checkpoint

When the agent's confidence is below threshold (per
`30-rich-ux-primitives.md` §2.5). The user chooses between two or
more options; the choice feeds back as a heuristic signal.

### 4.3 Review checkpoint

Before destructive or visible-to-others actions: delete, publish,
send, post. Always ask even if prior approval was granted. "Approval
once doesn't mean approval always" is load-bearing.

```
Review: create PR against upstream/main
  Files: src/core.rs, src/net.rs
  Commits: 1
  Labels: bug, refactor
  [V]iew diff  [E]dit body  [C]reate  [X]ancel
```

This is the "permission stands for the scope specified, not beyond"
rule applied.

## 5. Chain of custody

Every auditable action produces a `Custody` Engram (25 §8.2):

```
Custody {
    action:      ActionHash,           // what was done
    principal:   PrincipalId,          // who did it
    when:        Timestamp,
    authorized:  AuthzEvidence,        // role grant, confirm, escalation?
    why_heuristics: Vec<HeuristicId>,  // priors applied
    why_claims:  Vec<ClaimId>,         // research backing
    simulation:  Option<SimHash>,      // for blockchain / ops
    gates_passed: Vec<GateVerdict>,
    result:      Option<ResultHash>,
    witness:     Option<ChainWitness>, // Phase 2+
}
```

Every domain profile declares which actions require custody records
(25 §4 blockchain: every signed tx; 25 §6 ops: every production
write; 25 §3 research: every external fact-claim).

Custody is queryable:

```bash
roko custody list --action transfer --after 2026-04-01
roko custody show <action-hash>
roko custody verify <action-hash>   # re-check heuristic calibrations
roko custody export --signed > audit.jsonl
```

The chain witness integration (when Phase-2 lands) appends a
signature trail that cross-deployment auditors can verify
independently.

## 6. Plugin sandboxes

Per-tier rules (expanding 17 §8):

### 6.1 Tier 1 & 2 — pure data

No code runs from the plugin. Content is parsed as TOML/YAML/Markdown
and validated against a schema. A malicious tier-1 plugin can at most
propose a bad prompt; the safety layer catches it when the agent
tries to act on it.

### 6.2 Tier 3 — declarative tool manifest

Tool runs as subprocess or MCP server. Enforced:

- `cwd` fixed to a declared directory.
- `env` scrubbed to a declared whitelist.
- `args` templated against `TypedContext` with validation.
- `files_read`, `files_write` glob patterns enforced by pre-call
  path canonicalization + deny-by-default.
- `network` toggle — when false, tool runs in a network-less
  namespace (Linux) or without network entitlement (macOS).
- `timeout_ms` enforced; process killed on expiry.
- `role_allow` restricts which roles can invoke.

A single `safety.pre_call` and `safety.post_call` gate wraps every
tool invocation.

### 6.3 Tier 4 — native extension

Loaded as `cdylib` against the `roko-spi` ABI (17 §5.1). Enforced:

- Extension compiled against a frozen ABI version; incompatible
  ABI = refusal to load.
- Extension runs in the host process; *no runtime isolation*. The
  safety guarantee is purely "the author is trusted."
- Manifest declares permissions; the safety layer rejects calls
  outside declared scope.
- Crash in the extension doesn't propagate thanks to panic-catch
  wrappers around every SPI entry.

Tier 4 should require a signed manifest + reputation signal before
the installer accepts it (17 §7).

### 6.4 Tier 5 — WASM sandbox

Extension runs in a WASM host (17 §13). Enforced:

- CPU time limit per call (default 100 ms).
- Memory limit per instance (default 64 MB).
- No file system access.
- No direct network access. Only hostcalls.
- Hostcalls permission-checked against manifest before dispatch.
- Pulse rate and Substrate query rate limits per instance.

Violations kill the instance, publish `plugin.violation` Pulse,
flag the plugin in the UI as "violated sandbox." Repeated violations
auto-disable.

## 7. Taint tracking

Data from untrusted sources (web scrape, user paste, plugin output)
carries a `taint: Taint` field on the Engram:

```
enum Taint {
    None,
    UserInput,              // pasted prompt, uploaded file
    ExternalFetch(Source),  // HTTP GET, API call
    ThirdPartyPlugin(PluginId),
    LegacyImport,           // imported from another deployment
}
```

Propagation rule: any Engram whose *input* is tainted is itself
tainted. A Composer reads tainted Engrams and produces a tainted
composed prompt; an LLM turn reads the prompt and produces a tainted
output.

Safety gates at step 4 (per `05-loop-retold.md`) and at action points
read the taint and may:
- Require additional confirmation before acting.
- Refuse entirely for high-risk destinations (e.g. signing a
  blockchain tx with tainted recipient address — always escalates).
- Attach taint metadata to custody records so auditors can trace.

Taint is one-way: it only propagates; it doesn't "clean" without
explicit human action (a reviewer approves the output with sign-off).

## 8. Attestation

Some Engrams deserve cryptographic commitment:

```
Attestation {
    signer: PublicKey,
    signature: Ed25519Signature,
    signed_hash: ContentHash,
    timestamp: i64,
    level: AttestationLevel,
}

enum AttestationLevel {
    LocalAgent,       // signed by this agent's session key
    OrgRole,          // signed by a human-owned role key
    ChainWitness,     // committed to on-chain (Phase 2+)
}
```

Attestation is always opt-in per-kind. Defaults:

- GateVerdict: `LocalAgent` (low-friction auditability).
- Custody for destructive action: `OrgRole` (requires human sign-off).
- Heuristic commons contribution: `ChainWitness` (Phase 2+, for
  cross-deployment trust).

Verification is `roko attest verify <hash>` which walks the chain of
attestations along the Engram's lineage.

## 9. Network egress control

All outbound network calls go through a single shim:

```rust
pub trait Egress: Send + Sync {
    async fn get(&self, url: &Url, ctx: &SafetyCtx) -> Result<Response>;
    async fn post(&self, url: &Url, body: &[u8], ctx: &SafetyCtx) -> Result<Response>;
    fn allow(&self, url: &Url, principal: &Principal) -> bool;
}
```

Default implementation denies any URL whose host isn't on an allow-list.
The allow-list is populated from:

- Profile defaults (researcher: arxiv, semantic scholar; coder:
  crates.io, github; ops: cloud provider APIs).
- Plugin manifests (tier-3 can declare hosts they need).
- User-approved additions during session.

Every request logs source principal + URL + response status to
`network.egress.*` Pulses; the safety-events projection surfaces them
in the dashboard.

## 10. Secrets story

From `24-deployment-ux.md` §3, expanded:

- Secrets never appear in Engrams, Pulses, or logs.
- A `Secret` type wraps values; `Display` and `Debug` print `****`.
- Substrate and Bus both scrub `Secret`-typed fields on the way out.
- Secret rotation is observable: a `secrets.rotated` Pulse fires;
  consumers re-fetch.
- Plugin manifests cannot request `secrets_read` without tier-4 or
  tier-5 plus explicit operator approval.

The secret manager is trait-based; `SecretStore` is pluggable
(OS keychain, Vault, AWS Secrets Manager, 1Password CLI).

## 11. Multi-tenancy isolation

In single-server and clustered deployments (24 §1.2–§1.4):

- Each tenant has a namespace prefix on Bus topics, Substrate keys,
  and plugin scope.
- Cross-tenant data access denied at the Substrate/Bus layer, not at
  the UI layer. Defense in depth.
- Plugins declare whether they are `multi_tenant_aware` (can see
  across tenants) or `tenant_scoped` (default).
- Heuristic commons imports are quarantined per-tenant before
  general availability.

## 12. Evaluator roles and conflict of interest

Some roles evaluate other roles' outputs. Reviewer role evaluates
implementer; compliance role evaluates ops; replication agent
evaluates researcher. Conflict-of-interest rules:

- An agent cannot be both producer and sole reviewer of the same
  action in auto-mode.
- Ambiguous situations escalate to human.
- The `researcher → evaluator` separation from 16 §15 is a specific
  case of this rule.

Enforcement is policy-level — a `ConflictCheckPolicy` watches agent
assignments and flags self-review.

## 13. Threat model

What Roko assumes vs. what it doesn't:

**Assumed trusted**:
- The machine running the binary.
- The kernel crates and the default implementations.
- Signed tier-4 plugins from the registry once installed.
- OS-level secret storage.

**Assumed untrusted**:
- User prompts (could contain prompt injection).
- Remote LLM responses (could mislead tool use).
- Third-party content fetched via tools (web pages, arxiv PDFs,
  MCP server outputs).
- Unsigned tier-4/tier-5 plugins.
- Cross-tenant data in shared deployments.

**Outside the model** (operator must handle):
- Physical access to disk.
- Root compromise of the host.
- Upstream supply-chain attacks on crates.io.

Document the threat model in `docs/security/threat-model.md` so
reviewers know what's in scope.

## 14. Audit tooling

Commands that support a safety review:

```bash
roko custody list --after 7d --principal user:alice
roko custody verify --chain-witness
roko taint show <engram-hash>
roko secret audit              # who accessed what, when
roko plugin audit              # installed plugins, permissions, versions
roko attest list --level OrgRole
roko network log --tail 100
```

All of these hit the same Substrate + Bus primitives as the rest of
the system. Auditor sees the same truth the runtime sees.

## 15. Incident response

When something goes wrong, the combination of custody + attestation
+ taint + replay makes postmortems tractable:

1. Identify the problematic action's custody record.
2. Walk its lineage backward through contributing Engrams.
3. Check which heuristics + claims were cited; note their
   calibration at the time (not now — time-travel via replay).
4. Check taint sources.
5. Replay the decision with the same inputs to confirm
   reproducibility.
6. Publish a postmortem Engram (`Kind::Postmortem`) linked to the
   custody chain.
7. If the root cause was a heuristic, update its calibration; if
   a plugin, update its permissions; if a gate, tighten the pipeline.

This closes the loop: safety incidents become learning signals.

## 16. Staging

Safety is orthogonal to the kernel refactor (06), but it accretes
naturally:

- **Phase C.5**: extend today's safety layer to read from `TypedContext`
  and publish `safety.*` Pulses. One week.
- **Phase C.6**: custody records shipping for destructive actions.
  Two weeks.
- **Phase D**: plugin sandboxes (tier-3 hardening, tier-5 WASM host).
  Three weeks.
- **Phase E**: attestation, taint propagation, audit tooling.
  Three weeks.
- **Phase 2+**: chain witnesses on custody. Depends on `roko-chain`.

Total: two months of focused safety work for a production-grade
defensive spine. Some of it is already in place; this sequences the
rest.

## 17. Cross-references

- Role auth today: `crates/roko-agent/src/safety/`.
- Plugin SPI and tier sandboxes: `17-plugin-extension-architecture.md`.
- Domain-specific custody requirements: `25-domain-specific-agents.md` §8.2.
- Deployment implications (secrets, multi-tenancy): `24-deployment-ux.md`.
- Observability for safety events: `33-observability-telemetry.md` §4.
- Permission UX in each surface: `23-user-ux-running-agents.md` §10,
  `28-cli-parity-familiar-workflows.md` §18.
- Chain witness Phase-2 integration: `09-phase-2-implications.md` §1.
- Replication-ledger adversarial ingestion defense:
  `16-research-to-runtime.md` §15.

--- END 32-safety-sandbox-provenance.md ---

# Batch REF32 — Safety/sandbox/provenance spine across safety chapter

**Refinement source**: `tmp/refinements/32-safety-sandbox-provenance.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- `docs/11-safety/` — safety spine: role auth, tier sandboxes, pre/post checks, taint, attestation, chain-of-custody.
- `docs/00-architecture/05-provenance-and-attestation.md` — Custody record shape and attestation levels.
- `docs/00-architecture/26-cognitive-immune-system.md` — taint-propagation + detection.

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/32-safety-sandbox-provenance.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.


## Required vocabulary (verify)

The verify step greps for: `custody|sandbox|attestation`

At least one changed file must mention it (case-insensitive) before the
batch is considered complete.

## Cross-references

Dependents — downstream refinements assuming this one has propagated
into docs: REF33

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
- Commit ready with message `refinements(REF32): Safety/sandbox/provenance spine across safety chapter`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
