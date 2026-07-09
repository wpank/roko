# Refinements Batch REF14

Run id: run-20260416-221511
Attempt: 1
Model: gpt-5.4
Reasoning: high
Refinement source: /Users/will/dev/nunchi/roko/roko/tmp/refinements/14-worldview-validation.md
Target docs (candidates): docs/05-learning/ docs/00-architecture/INDEX.md docs/06-neuro/

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

Suggested parallel split for batch `REF14`:

- worker: add/update files under `docs/05-learning/` to describe the Heuristic
  type, Calibrator, Worldview clustering, Dissonance detection.
- worker: update `docs/06-neuro/` where distilled knowledge is discussed.
- worker: update `docs/00-architecture/INDEX.md` to link the heuristic chapter.

## Canonical refinement source

This is the verbatim refinement proposal that this batch must propagate
into `docs/`. Treat it as the authoritative source. Do not edit this
file; only edit docs under `docs/`.

--- BEGIN 14-worldview-validation.md ---

# Worldview Validation from Lived Experience

> **TL;DR**: Agents don't just accumulate facts — they accumulate
> *heuristics*, *priors*, and *mental models*. These are the most
> valuable Engrams in the system and the most dangerous if stale.
> This doc proposes a structured type — `Heuristic` — with
> explicit pre/post conditions, a track record, and a calibration
> score. Every episode either reinforces, violates, or extends a
> heuristic. Over time, worldviews emerge from the co-citation
> network of heuristics that hold up under lived experience.
> Nothing in the literature does this for code agents.

> **For first-time readers**: Roko today stores distilled knowledge as
> playbooks — concrete sequences of actions that worked before.
> Heuristics are more abstract: rules of thumb with preconditions, a
> prediction, and a track record of confirmations vs violations.
> Worldviews cluster heuristics that co-occur in successful episodes.
> This doc is the most user-facing of the learning refinements — it
> produces externalized, inspectable "beliefs" the user can browse,
> challenge, and export. Read 11 (HDC) and 12 (demurrage) first.

## 1. What's missing in playbooks

Roko's current "learned knowledge" is expressed as *playbooks* in
`roko-learn`: concrete distilled sequences of actions that worked
before. Playbooks are useful but are:

- **Over-specific**: bound to particular tools and file layouts.
- **Brittle**: one changed path breaks the whole playbook.
- **Flat**: no notion of *why* they worked, only *that* they did.
- **Unattributable**: a playbook's success can't be traced back to
  a specific belief that was validated.

Humans don't operate with playbooks alone. They operate with
**heuristics**: compact, often-wrong-but-useful rules of thumb with
a sense of when to apply them. "If the test is flaky, add logging
before touching the logic." "If the build fails after a merge,
check lockfiles first." These are *meta* — they tell you what to
do, not what to do it with.

## 2. The `Heuristic` type

A first-class Engram variant:

```rust
pub struct Heuristic {
    pub id: Uuid,

    /// Natural language, canonical form.
    pub claim: String,

    /// Conditions under which the claim applies.
    /// Structured so they can be matched against a candidate
    /// situation automatically.
    pub preconditions: Vec<Predicate>,

    /// Expected outcome if the claim holds and preconditions match.
    pub prediction: Predicate,

    /// HDC fingerprint of the (preconditions + claim) tuple.
    /// Enables similarity search in heuristic space.
    pub fingerprint: HdcVector,

    /// Calibration record, incrementally maintained.
    pub calibration: Calibration,

    /// Parent heuristics this specializes or generalizes.
    pub lineage: Vec<HeuristicId>,

    /// The episodes where it applied (as lineage).
    pub receipts: Vec<EpisodeHash>,
}

pub struct Calibration {
    pub trials: u32,              // times preconditions matched
    pub confirmations: u32,       // times prediction held
    pub violations: u32,          // times prediction failed
    pub brier_score: f64,         // calibration quality
    pub last_trial_at: Timestamp,
    pub confidence_interval: (f64, f64), // Wilson CI
}
```

`Predicate` is a small, extensible enum that can be matched against
a situation Engram:

```rust
pub enum Predicate {
    LanguageIs(Language),
    FileMatches(Glob),
    ToolAvailable(ToolId),
    GateRecentlyFailed(GateId),
    AgentRoleIs(Role),
    Custom(Box<dyn PredicateFn>),
    And(Vec<Predicate>),
    Or(Vec<Predicate>),
    Not(Box<Predicate>),
    SimilarTo { fingerprint: HdcVector, threshold: f64 },
}
```

## 3. The lifecycle of a heuristic

### 3.1 Birth

Heuristics are born three ways:

1. **Distilled** from a cluster of episodes by the existing playbook
   distillation in `roko-learn`, but emitting a *precondition →
   prediction* rather than a sequence of steps.
2. **Stated** by an agent explicitly: "I think X, because Y." The
   agent writes a Heuristic Engram as a tool-call; other agents see
   it; it enters the calibration loop.
3. **Imported** from research (see `16-research-to-runtime.md`).

### 3.2 Test

Every episode is a potential test:

- Before executing, the Composer scans for heuristics whose
  preconditions match the current situation (HDC similarity +
  predicate evaluation).
- Matching heuristics are injected into the system prompt as
  *advisory claims*.
- The agent is free to use, ignore, or contradict them.
- After the episode, a Policy inspects outcomes and updates each
  heuristic's calibration.

```rust
pub trait Calibrator {
    fn score(&self, h: &Heuristic, episode: &Episode) -> Verdict;
}

pub enum Verdict {
    Confirmed,                // prediction held
    Violated,                 // prediction failed
    Irrelevant,               // preconditions didn't actually match
    Refined(Heuristic),       // new, narrower version learned
    Generalized(Heuristic),   // new, broader version learned
    Refuted,                  // so badly wrong we should retire it
}
```

### 3.3 Adjust

Calibration updates are a conjugate Beta update:

```text
confirmations ← confirmations + 1  if Confirmed
violations    ← violations + 1     if Violated
```

Confidence interval recomputed via Wilson score interval. The
heuristic's *effective weight* in prompt selection is a function of
confidence-lower-bound — optimistic-under-uncertainty. Young
heuristics with two confirmations are *exactly as useful* as
veterans with 200 until one fails.

### 3.4 Retire

Heuristics don't get deleted. Their balance (see `12-knowledge-demurrage.md`)
decays, they move to cold tier, but their hash remains valid so any
Engram that cited them still has a resolvable lineage. *History is
preserved, attention is not owed*.

### 3.5 Evolution

When a heuristic is violated, the Calibrator can spawn a *refined*
version that adds a new precondition ("except when X is true"). Over
time, a heuristic tree emerges: a rough root, specializations for
discovered exception domains, generalizations when a pattern shows
up across domains. This is Quinlan's ID3 in a different dress, but
on a live stream rather than a batch.

## 4. Worldviews as co-citation clusters

A **worldview** is not a single heuristic — it's a set of
mutually-citing heuristics that co-occur in successful episodes.

```rust
pub struct Worldview {
    pub id: Uuid,
    pub core_heuristics: Vec<HeuristicId>,
    pub coherence_score: f64,     // avg pairwise HDC similarity
    pub effectiveness_score: f64, // avg calibration of members
    pub domain_fingerprint: HdcVector, // where this worldview applies
}
```

Worldviews emerge from community detection on the heuristic
citation graph. No one declares a worldview; it's *observed* from
the statistics. Worldview X scores well on web-frontend tasks;
worldview Y scores well on distributed-systems tasks. When a new
task comes in, the Router picks the worldview whose
`domain_fingerprint` is closest and injects its heuristics.

This generalizes *personality* across agents without requiring
hand-engineered personas.

## 5. Multiple worldviews, deliberately

One of the key moves from `13-collective-intelligence-c-factor.md`:
**diversity of opinion is a load-bearing property**. Therefore: keep
more than one active worldview even when one is currently dominant.

- **Main worldview**: highest effectiveness on recent tasks.
- **Challenger worldview**: second-highest, deliberately invoked on
  some fraction of tasks (ε-greedy, Thompson-style).
- **Niche worldviews**: specialists preserved in cold tier, thawed
  when domain fingerprints match.

This is the structural answer to "how do we avoid monoculture" posed
in doc 13 — the answer is *keep your priors plural on purpose*.

## 6. Lived experience, not stated belief

The critical difference between this and "LLM-stated preferences":
**every heuristic's calibration comes from actual outcomes, not from
the agent's self-report**. An agent can confidently claim X; if X
fails five times in a row, the heuristic's calibration drops
regardless of how confident the claim was. The substrate is empirical
ground truth; the agent is a hypothesis generator.

This is the central philosophical commitment: **assert beliefs, test
them against Pulses, preserve outcomes, update calibrations.** The
Bus is the reality-check channel.

## 7. Perspective-taking: heuristics about heuristics

Agents should model *other agents' heuristics*. A
`peer_heuristic_model`:

```rust
pub struct PeerModel {
    pub agent: AgentId,
    pub believed_heuristics: Vec<(HeuristicId, f64)>, // with weight
    pub calibration: Calibration, // how well we predict *them*
}
```

This directly feeds `peer_prediction_accuracy` in the c-factor
computation (`13`). Being a good team member *means* modeling your
teammates' priors accurately. It's social perceptiveness made
algorithmic.

## 8. Dissonance detection

When the active Heuristic set contains two that would predict
different outcomes for the current situation, that's a *dissonance
event*. Dissonance is surfaced explicitly:

```rust
pub struct Dissonance {
    pub heuristics: [HeuristicId; 2],
    pub predictions: [Predicate; 2],
    pub situation: SituationHash,
}
```

Dissonance events are high-information-content. The Policy layer
can:

1. Spawn a decisive agent to act and collect ground truth.
2. Use the outcome to update both heuristics' calibrations.
3. Prefer dissonance-resolving tasks in scheduling (active learning).

This is Festinger's cognitive dissonance operationalized as a
learning signal. Most agent systems *hide* contradictions by
picking one; we *surface* them because they're where learning
happens fastest.

## 9. Externalization: the heuristic library

The heuristic store is queryable via CLI and HTTP:

```bash
roko heuristic list --confidence 0.7
roko heuristic show <id>
roko heuristic similar <situation>
roko heuristic stats         # calibration health
roko heuristic export        # JSONL for analysis
```

This is what makes Roko *inspectable*. A user can read the agent's
current beliefs, see which ones are battle-tested, see which ones
are recent hypotheses. You get a literal answer to "what does my
agent think it knows?"

No other framework offers this. It's net-new product surface area.

## 10. Sharing heuristics across deployments

Because heuristics are content-addressed Engrams with calibration
metadata, they're *exportable and importable* between Roko
instances:

```bash
roko heuristic export --confidence 0.8 > my-heuristics.jsonl
# on another machine:
roko heuristic import --trust 0.5 < my-heuristics.jsonl
```

Imported heuristics enter with their remote calibration and a
configured trust factor; they get revalidated in the new context
before they influence decisions. This is the primitive for a
**heuristic commons** — a git-like exchange of empirically-validated
agent knowledge.

Phase 2+ chain witnesses could be applied here: a heuristic with
1,000 confirmations across 50 deployments and a chain signature
from each is *very* trustworthy. A heuristic someone typed in last
week is not.

## 11. Why this is different from RAG

RAG retrieves facts. Heuristic-validation retrieves *operating
principles with a track record*. The retrieval isn't "find me
similar text" but "find me the priors that held up last time I was
in a situation like this." That's a categorically different kind of
memory than vector-store RAG, and it's enabled by demurrage +
lineage + HDC + active inference all being in the same system.

## 12. Minimal viable implementation

Fast path:

1. Add `Heuristic` Engram variant + `Predicate` enum. Two days.
2. Wire heuristic retrieval into the Composer (prompt injection). One day.
3. Wire Calibrator into the post-episode Policy pass. Two days.
4. Implement co-citation-based Worldview clustering. One week.
5. CLI surface (`roko heuristic *`). Three days.
6. Dissonance detection and active-learning scheduler. One to two weeks.

Biggest win per engineering-day is steps 1–3. They produce a
learnable library of priors without any further infrastructure.

## 13. Heuristic-level granularity: a worked example

Two heuristics about flaky tests:

```yaml
- id: h.flaky.42
  claim: "When a test is flaky in CI but passes locally, check for \
         timing or resource ordering assumptions."
  preconditions:
    - LanguageIs: Rust
    - GateRecentlyFailed: { gate: unit, intermittent: true }
  prediction:
    Predicate: TestTiminaRaceSuspected
  lineage: []
  calibration:
    trials: 41, confirmations: 28, violations: 13, brier_score: 0.21
  receipts: [ep_8712, ep_9311, ep_9822, ...]

- id: h.flaky.57  (refined child of h.flaky.42)
  claim: "When a Rust test is flaky in CI and uses tokio::test, \
         check the runtime flavor first."
  preconditions:
    - LanguageIs: Rust
    - GateRecentlyFailed: { gate: unit, intermittent: true }
    - Similar: { fingerprint: hdc(tokio_runtime_flavor), threshold: 0.7 }
  prediction:
    Predicate: TokioRuntimeFlavorMismatchSuspected
  lineage: [h.flaky.42]
  calibration:
    trials: 7, confirmations: 6, violations: 1
  receipts: [ep_9822, ep_9911, ...]
```

The parent is general; the child specializes. Both stay in the
library. Retrieval prefers the child (more specific precondition)
when it matches; falls back to the parent otherwise. This is the
ID3-on-a-stream pattern from §3.5 made concrete.

## 14. Meta-heuristics — heuristics about heuristics

The same framework applies one level up. A meta-heuristic:

```yaml
- id: mh.distill.01
  claim: "When refining a heuristic after a violation, the new \
         precondition should be observable *before* the action, \
         not inferred only from the outcome."
  preconditions:
    - Event: HeuristicRefinementProposed
  prediction:
    Predicate: RefinedHeuristicWillImproveCalibration
```

Meta-heuristics drive the Calibrator itself. A meta-heuristic with
low calibration says the refining strategy is broken; the system
adjusts how it produces child heuristics.

This is where Minsky's "society of mind" (§6.3 of doc 10) lands:
a layered architecture of predictors about predictors, each with
its own calibration. No magic; just recursive application of the
same primitive.

## 15. Permission gradient for user actions

Not all user-driven heuristic actions are equal. A safety gradient:

| Action | Auto? | Permission needed |
|---|---|---|
| Browse heuristics | Yes | None |
| Stake on a heuristic | Yes | None (it's play money) |
| Add a receipt manually | Yes | None |
| Edit a heuristic's *claim* | Ask | Per-session confirm |
| Edit a heuristic's *preconditions* | Ask | Per-session confirm |
| Retire a heuristic | Ask | Per-session confirm |
| Challenge (force recalibration on next 10 trials) | Yes | None |
| Import from commons (unsigned) | Ask | Explicit confirm |
| Import from commons (signed, >10 deployments) | Auto | None |
| Export to commons | Ask | Explicit confirm |

See `32-safety-sandbox-provenance.md` for the full permission model
the CLI/TUI/Web UI apply these to.

## 16. Interaction with domain profiles

Domain profiles from `25-domain-specific-agents.md` seed heuristic
libraries. A coding profile starts with ~30 canonical coding
heuristics; a research profile starts with the research claims from
`16-research-to-runtime.md` §8; a blockchain profile starts with
historical-exploit heuristics (reentrancy patterns, etc.).

When two profiles are installed together, their heuristics coexist.
Calibration is per-heuristic, not per-profile, so a cross-domain
heuristic that actually pays off in both domains gets reinforced
from both — exactly what a general principle should do. This is
the structural reason profiles *compose* cleanly (25 §10).

--- END 14-worldview-validation.md ---

# Batch REF14 — Heuristics + falsifiers as new learning sub-chapter

**Refinement source**: `tmp/refinements/14-worldview-validation.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- `docs/05-learning/` — add Heuristic-type chapter(s): type, Calibrator, Worldview clustering, Dissonance.
- `docs/06-neuro/` — where distilled knowledge is discussed; cross-reference.
- `docs/00-architecture/INDEX.md` — link the new heuristic material.
- `docs/00-architecture/19-compositional-kinds.md` — add Heuristic to kinds list if applicable.

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/14-worldview-validation.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.


## Required vocabulary (verify)

The verify step greps for: `heuristic|falsifier|worldview`

At least one changed file must mention it (case-insensitive) before the
batch is considered complete.

## Cross-references

Dependents — downstream refinements assuming this one has propagated
into docs: REF15, REF16, REF19, REF25, REF31

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
- Commit ready with message `refinements(REF14): Heuristics + falsifiers as new learning sub-chapter`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
