# Refinements Batch REF16

Run id: run-20260416-221511
Attempt: 1
Model: gpt-5.4
Reasoning: high
Refinement source: /Users/will/dev/nunchi/roko/roko/tmp/refinements/16-research-to-runtime.md
Target docs (candidates): docs/21-references/ docs/05-learning/ docs/00-architecture/INDEX.md

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

Suggested parallel split for batch `REF16`:

- worker: add/update files under `docs/21-references/` with the paper →
  claim → heuristic → trial → calibration pipeline and replication ledger.
- worker: update `docs/05-learning/` where claim-based parameters are referenced.
- worker: update `docs/00-architecture/INDEX.md` to link the replication
  chapter.

## Canonical refinement source

This is the verbatim refinement proposal that this batch must propagate
into `docs/`. Treat it as the authoritative source. Do not edit this
file; only edit docs under `docs/`.

--- BEGIN 16-research-to-runtime.md ---

# Research-to-Runtime Pipeline

> **TL;DR**: Roko should consume academic and industry research
> continuously, not as a one-time inspiration. This doc proposes a
> typed pipeline — `Paper → Claim → Heuristic → Trial → Calibration`
> — where every cited result becomes a testable hypothesis in the
> running system. Papers are Engrams; claims are candidate
> Heuristics; trials are episodes; calibrations are published.
> Over time, the system builds an empirical map of which academic
> findings hold up *in its actual deployment*. This is
> evidence-based engineering for agent runtimes.

> **For first-time readers**: Roko already draws on research — HDC from
> Kanerva, active inference from Friston, c-factor from Woolley, demurrage
> from Gesell. Today that influence is folklore: someone read a paper and
> wrote some code. This doc promotes *the paper itself* to a first-class
> Engram, *the claim* to a testable hypothesis with a falsifier, and
> *the system's own trials* to continuous re-replication. Read 14
> (heuristics) first; this is heuristic infrastructure specialized to
> academic provenance.

## 1. The state of research-in-code today

Roko already has research-inspired primitives scattered through the
codebase:

- HDC from Kanerva 2009.
- Active inference / FEP from Friston 2006.
- Predictive processing from Clark 2013.
- Stigmergy from Grassé 1959.
- c-factor from Woolley 2010.
- Demurrage from Gesell 1916.
- Bandits from Robbins/Auer.
- Playbook distillation echoes Sutton/Schmidhuber meta-learning.

But each is *imported as folklore*: an engineer reads a paper, writes
some Rust, the paper's specific claims and their calibration context
are lost. When the system behaves oddly, nobody can check whether
we're violating a precondition the paper stated.

The pipeline proposed here keeps papers and their claims *alive and
testable*.

## 2. Paper as Engram

```rust
pub struct Paper {
    pub id: Uuid,
    pub doi: Option<String>,
    pub arxiv: Option<String>,
    pub title: String,
    pub authors: Vec<String>,
    pub year: u16,
    pub venue: Option<String>,
    pub abstract_: String,
    pub full_text_hash: Option<EngramHash>,

    /// HDC fingerprint of the abstract + title.
    pub fingerprint: HdcVector,

    /// Claims we extracted or asserted from this paper.
    pub claims: Vec<ClaimId>,

    /// How we've decided to trust this paper.
    pub provenance: PaperProvenance,
}

pub struct PaperProvenance {
    pub source: Source,       // arxiv, nature, blog, etc
    pub citation_count: Option<u32>,
    pub venue_tier: Option<VenueTier>,
    pub replication_status: ReplicationStatus,
    pub our_notes: Option<String>,
}
```

Paper Engrams live in the same Substrate as everything else. They
get content-addressed hashes. Heuristics and episodes can cite them
in lineage.

## 3. Claim as testable hypothesis

A `Claim` is a sharper, structured version of a sentence from the
paper:

```rust
pub struct Claim {
    pub id: Uuid,
    pub paper: PaperId,
    pub quote: String,

    /// Structured restatement.
    pub hypothesis: Hypothesis,

    /// What would refute this claim in our context.
    pub falsifier: Predicate,

    /// Conditions under which the paper says the claim applies.
    pub context: Vec<Predicate>,

    /// Effect size reported in the paper.
    pub effect_size: Option<EffectSize>,

    /// Our empirical evaluation so far.
    pub calibration: Calibration,
}

pub enum Hypothesis {
    Causal { cause: Predicate, effect: Predicate, sign: Sign },
    Statistical { distribution: Expr, parameters: Vec<f64> },
    Algorithmic { invariant: String, guarantee: Expr },
    Architectural { structure: Predicate, property: Predicate },
}
```

Claims are *authored* — a human or agent reads a paper and writes
one down. Extraction can be semi-automated: the Composer can draft
claims from an abstract for review. The human-in-the-loop step is
the falsifier — stating *what would prove this wrong* is the work
Popper demanded and LLMs do poorly unsupervised.

## 4. Claim → Heuristic lifting

When a Claim reaches sufficient structure, it *becomes* a Heuristic:

```rust
impl From<Claim> for Heuristic {
    fn from(c: Claim) -> Heuristic {
        Heuristic {
            claim: c.quote,
            preconditions: c.context,
            prediction: c.hypothesis.into_predicate(),
            lineage: vec![], // citation captured separately
            calibration: c.calibration,
            ..default()
        }
    }
}
```

The same lifecycle from `14-worldview-validation.md` applies:
trials, confirmations, violations, refinement, retirement. The only
difference is lineage points back to a Paper Engram, and the
calibration diverges from the paper's reported effect over time —
this divergence *is the interesting signal*.

## 5. The replication ledger

For each paper-derived claim, Roko maintains a replication ledger:

```rust
pub struct ReplicationLedger {
    pub claim: ClaimId,
    pub paper_effect: f64,       // what the paper reported
    pub our_effect: f64,         // what we observe
    pub our_n: u32,              // trials in our context
    pub divergence_ci: (f64, f64), // confidence interval of the gap
    pub status: ReplicationStatus,
}

pub enum ReplicationStatus {
    Untested,
    Insufficient(u32),  // too few trials
    Replicates,         // effect within CI
    PartialReplicates,  // same sign, smaller effect
    FailsToReplicate,
    ContextDependent,   // replicates in some situations, not others
}
```

This makes "does this research hold up on our stack?" a structured
dashboard query. Nothing in the agent-framework space does this.

## 6. Cited research as first-class config

Instead of:

```rust
const CASCADE_EPSILON: f64 = 0.1; // "from auer et al"
```

We prefer:

```rust
CascadeRouter::new()
    .epsilon_from(claim!["auer2002", "epsilon_greedy", "default_0.1"])
    .with_fallback(0.1)
```

The `claim!` macro resolves to a Claim ID at build time; at runtime,
if the claim's calibration has drifted enough, a signal is emitted:
"cascade-router-01 is using a parameter whose source claim has
failed to replicate in 87 recent trials." Engineering decisions
become *traceable* and *self-auditing*.

## 7. Research sources and ingestion

Three ingestion lanes:

### 7.1 Manual

A human (or agent) reads a paper, creates the Paper Engram, drafts
Claims. Highest-quality ingestion; appropriate for foundational
work.

### 7.2 Agent-curated

An agent in "researcher" role crawls a source (arxiv daily digest,
Papers With Code trending), drafts Paper+Claim Engrams, publishes
them to a Bus topic `research.candidate`. Other agents review and
either promote to `research.approved` or reject.

### 7.3 Watchdog

A Watchdog subscribes to a claim's falsifier Predicate *across all
episodes*. When the falsifier matches an observed outcome, the
Watchdog publishes `claim.violated` and triggers recalibration.
Passive monitoring; zero operator overhead.

## 8. A curated starter kit

The proposal includes importing ~40 foundational claims at launch:

- Kanerva 2009 on HDC capacity and near-orthogonality.
- Friston 2006 on free-energy minimization.
- Woolley 2010 on c-factor predictors.
- Sutton 1988 on temporal-difference learning.
- Robbins 1952 and Auer 2002 on bandits.
- Hanson 1999 on prediction markets.
- Axelrod 1984 on cooperation / tit-for-tat.
- Janis 1972 on groupthink symptoms.
- Dehaene 2020 on consciousness and global workspace.
- Holland 1995 on complex adaptive systems.
- Kahneman 2011 on System 1/2 and bias catalog.
- Sapolsky 2004 on stress and decision-making (for pacing).
- Clark 2013 on predictive processing.
- Surowiecki 2004 on wisdom-of-crowds conditions.
- Weick 1995 on sensemaking.
- Ostrom 1990 on commons governance (for the heuristic commons).
- Mead 1934 on role-taking (for peer-prediction).
- Simon 1956 on bounded rationality.
- Gesell 1916 on demurrage.
- Hofstadter 1979 on strange loops (for self-modeling).

Each gets a Paper Engram and 1–3 Claims, with a falsifier stated
explicitly. This becomes the starter heuristic library every new
deployment inherits. Calibration against each deployment's reality
takes over from there.

## 9. Provenance as a first-class quality signal

When an agent uses a heuristic in a prompt, the prompt includes its
provenance:

```
[heuristic] When tests are flaky, add logging before touching logic.
  source:   rooted in Kernighan & Pike 1999 §5.2
  our n:    41 trials, 28 confirmations
  paper CI: not quantified
  our CI:   (0.54, 0.79)
```

This is *radical transparency* about what the agent believes and
why. Most LLM agent systems are opaque; Roko's are legible. The
legibility itself is a product feature — you can audit, review,
correct.

## 10. Refuting a paper

If Roko's calibration strongly diverges from a paper's reported
effect, that's publishable information. The ReplicationLedger can
export to a standard format (a markdown template with CIs, trial
counts, context specification). Someone running Roko can contribute
to meta-science just by running the system.

This is downstream but is a genuine possibility — *a coding
assistant that contributes to the replication crisis in a positive
direction*. None of the agent frameworks can make this claim.

## 11. Integration with the chain (Phase 2)

When a claim's replication is chain-witnessed across many
deployments, it becomes a *consensus claim*. Consensus claims carry
very high trust and anchor a shared scientific substrate across the
Roko ecosystem. This is `roko-chain` in the service of empirical
knowledge rather than financial transactions — the same primitive
applied with a very different flavor.

## 12. Minimal viable implementation

1. Paper + Claim Engram types. One day.
2. Starter kit of 20 canonical papers, manually authored. Three days
   of librarian work.
3. Research-role agent that reviews arxiv daily. One week.
4. ReplicationLedger + export. Three days.
5. Watchdog hooks for falsifier monitoring. One week.
6. Claim-resolved config parameters (`claim!` macro). Two days.
7. Prompt-provenance injection. One day.

This whole module is a couple of eng-weeks and establishes a
capability no other agent system has: *living research*.

## 13. What makes a good falsifier

The falsifier is the load-bearing part of a Claim. It separates
"inspirational reading" from "testable hypothesis." Good falsifiers
share three properties:

1. **Observable from runtime signals.** A falsifier that requires
   a lab experiment Roko can't run is useless. Rewrite as a
   condition on Engrams, Pulses, or metrics.
2. **Time-bounded.** A prediction that takes 10 years to fail isn't
   useful to a runtime that iterates daily. Frame falsifiers as
   "over the next N trials / N days, [observable] should hold."
3. **Discriminating.** A falsifier that passes even when the claim
   is wrong is noise. Write the falsifier so the claim has to
   actually work for it to pass.

Bad: *"Epsilon-greedy will converge."*
Good: *"Over the next 500 arm pulls, the cumulative regret should be
bounded by 2·ε·t·|A|·log(t); check at n=100, n=250, n=500. If any
checkpoint exceeds by > 3σ, flag."*

The good version is a literal statistical test the Watchdog can run.

## 14. Replication contract format

For ledger exports (to contribute to external meta-science), a
canonical format. Markdown + front-matter so humans and parsers
both handle it:

```markdown
---
claim_id: c.kanerva2009.orthogonality
paper_doi: 10.1007/s12559-009-9009-8
paper_effect: "Two random 10,000-bit vectors have cosine similarity ~0 ± 0.01"
our_effect: 0.0097 ± 0.004
our_n: 1_000_000
roko_version: "2.3.1"
context:
  vector_dim: 10240
  encoder: default_v1
  deployment_profile: coding
status: replicates
first_observed: 2026-03-01
last_observed: 2026-04-14
---

## Notes

We observe expected orthogonality within 95% CI of paper's claim
across a random sample of 1M vector pairs from our production
Substrate. No dependence on kind or body type detected. Deployment
profile does not alter the result.
```

This is a format other research groups can import, parse, and
cross-check against their own replications. It's the minimal
infrastructure for a decentralized replication network.

## 15. The ingestion conflict-of-interest problem

If Roko's own agents ingest and evaluate research, there's a
conflict: Roko might preferentially confirm research that justifies
Roko. Mitigations:

1. **Separate ingestion agents from calibration agents.** The
   researcher role ingests and proposes; a separate evaluator role
   maintains the ledger. The evaluator is forbidden from reading
   the ingested paper directly — only from observing runtime
   behavior.
2. **Adversarial prompts.** Ingest known-false papers (retracted,
   failed-to-replicate) into the starter kit. If Roko's evaluator
   confirms them anyway, the evaluator is broken.
3. **Human review of high-stakes claims.** Claims that would
   change system defaults (e.g. "reduce epsilon from 0.1 to 0.05")
   require human sign-off before taking effect, even if replication
   signals support.

The third is a permission-gradient item; see
`32-safety-sandbox-provenance.md` §4.

## 16. Research-driven roadmap refinement

A distinctive loop this doc enables: *the replication ledger drives
architectural decisions*. Two examples:

- If Kanerva's orthogonality claim fails to replicate in a specific
  Substrate backend, that's a signal HDC encoding is broken in that
  backend; file an issue against `roko-hdc`.
- If Woolley's c-factor predictors fail to replicate in Roko's
  multi-agent cohorts, it suggests either the agent analog doesn't
  map (revise the analogy) or the cohort is too small; adjust
  experimental design.

This closes the loop: research informs the system, the system tests
the research, results inform the next round of architectural work.
`35-consolidated-roadmap.md` has a placeholder for "replication-ledger-
driven" roadmap items that emerge organically over months.

--- END 16-research-to-runtime.md ---

# Batch REF16 — Research-to-runtime pipeline across references + learning

**Refinement source**: `tmp/refinements/16-research-to-runtime.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- `docs/21-references/` — Paper/Claim/Heuristic/Trial/Calibration pipeline + replication ledger format.
- `docs/05-learning/` — claim-resolved config parameters, `claim!` macro.
- `docs/00-architecture/INDEX.md` — link the replication chapter.

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/16-research-to-runtime.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.


## Required vocabulary (verify)

The verify step greps for: `replication ledger|claim|falsifier`

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
- Commit ready with message `refinements(REF16): Research-to-runtime pipeline across references + learning`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
