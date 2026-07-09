# Refinements Batch REF09

Run id: run-20260416-221511
Attempt: 1
Model: gpt-5.4
Reasoning: high
Refinement source: /Users/will/dev/nunchi/roko/roko/tmp/refinements/09-phase-2-implications.md
Target docs (candidates): docs/08-chain/ docs/10-dreams/ docs/13-coordination/ docs/16-heartbeat/ docs/00-architecture/24-cross-section-integration-map.md

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

Suggested parallel split for batch `REF09`:

- worker: update `docs/08-chain/` INDEX + key files with ChainBus vs
  ChainSubstrate split.
- worker: update `docs/10-dreams/` with Substrate scan + Bus-subscription
  inputs wording.
- worker: update `docs/13-coordination/` (stigmergy) to phrase pheromone
  deposit + mesh.pheromone Bus topic.
- worker: update `docs/16-heartbeat/` with HeartbeatPolicy-publishes-Pulses
  framing.

## Canonical refinement source

This is the verbatim refinement proposal that this batch must propagate
into `docs/`. Treat it as the authoritative source. Do not edit this
file; only edit docs under `docs/`.

--- BEGIN 09-phase-2-implications.md ---

# Phase 2+ Implications

> **TL;DR**: The two-fabric model makes chain, dreams, coordination,
> and mesh land as swap-in Bus/Substrate backends, not as rewrites.
> Stigmergy becomes a literal sentence. The HTTP control plane and
> per-agent sidecar stop being special cases and become Bus
> consumers. Multi-agent collectives become pub/sub topologies.

> **For first-time readers**: Phase 2+ in Roko's roadmap refers to the
> chain layer (on-chain coordination), dreams (offline consolidation),
> mesh (inter-agent p2p), and a few cross-cuts (Daimon affect, heartbeat
> clock). Today they are partial or stubbed. This doc walks each Phase-2
> subsystem and shows how the two-fabric kernel from 01–08 makes each
> one *smaller* rather than adding to the architectural surface area.

## 1. Chain (Korai / Daeji — Phase 6)

`docs/00-architecture/08-chain-layer.md` describes Roko's chain
integration as shared on-chain state for agent coordination, with
three transport needs: **storing** signed Engrams (transactions,
attestations), **reading** shared knowledge (insights, bounties), and
**reacting** to on-chain events.

In the current architecture, these three needs are lumped together
into a single `ChainSubstrate` (see `crates/roko-core/src/traits.rs`
comments — "ChainSubstrate — on-chain state via RPC"). But reading
on-chain state is a query, and reacting to on-chain events is
fundamentally a subscription — those are different fabrics.

With two fabrics:

- **`ChainSubstrate`** stores and queries durable on-chain Engrams
  (transactions, attestations, insights, bounties, pheromones). It
  already makes sense as a Substrate.
- **`ChainBus`** (new) maps event-log topics to Bus topics. A smart
  contract emits a `Deposited(agent, amount)` log; the ChainBus
  turns it into a `chain.deposit.emitted` Pulse. Subscribers in
  `roko-learn`, `roko-conductor`, and dashboards see it the same
  way they see any other Pulse.

This is the clean mapping. Without it, every chain-event consumer
has to poll `ChainSubstrate` or set up its own RPC subscription — a
repetition of the polling-vs-streaming bug that's already P0 in
`tmp/ux-followup/12-tui-event-parity.md`, just at a different layer.

## 2. Dreams (offline consolidation — Phase 5C)

`docs/00-architecture/10-dreams.md` describes Dreams as a Delta-speed
(hours-scale) loop that consolidates recent Engrams into higher-tier
knowledge. It's scaffold-only today (per `docs/STATUS.md`).

In the one-noun model, Dreams has to walk the Substrate to find
candidate Engrams for consolidation. It's a polling loop.

In the two-fabric model, Dreams has two inputs:

1. **Substrate scan** — still the primary source, because
   consolidation is deliberate and wants completeness.
2. **Bus subscription** — to `substrate.engram.stored` (emitted by
   the Substrate when new durable Engrams land). This makes Dreams
   reactive: it can wake up when a threshold of new content is
   available rather than polling on a fixed schedule. That matters
   because Delta-speed doesn't mean fixed-cadence; it means
   "slower than Gamma/Theta" — and "slower" can be event-triggered.

Dreams also emits consolidated `Kind::Insight` and `Kind::Heuristic`
Engrams. In the two-fabric model it emits both the Engram (to
Substrate) *and* an `engram.promoted` Pulse (to Bus) so the Composer
at L2 can react and update its enrichment heuristics without
re-querying.

## 3. Coordination / Stigmergy — Phase 13

`docs/00-architecture/13-coordination.md` describes stigmergic
coordination — agents leaving pheromone traces that other agents
follow. Grassé's original stigmergy concept (1959) is *shared
environmental state as indirect communication*.

In the two-fabric model, stigmergy is a literal one-liner:

> Pheromones are Engrams persisted to a shared Substrate (chain or
> mesh) with Ebbinghaus decay. Agents deposit pheromones by
> `substrate.put`; they detect them by `substrate.query` and/or by
> subscribing to `mesh.pheromone.deposited` on the Bus.

No new mechanism needed. The one-noun model made this awkward
because "put a signal and also somehow tell nearby agents it's
there" required custom plumbing. The two-fabric model separates
*depositing* (Substrate) from *alerting* (Bus) — which is exactly
the ant-trail dynamic.

## 4. HTTP control plane — already Bus-shaped

`crates/roko-serve/` exposes ~85 routes plus SSE and WebSocket
streams. Today the WebSocket/SSE endpoints fan out internal
broadcast channels through ad-hoc conversions. In the two-fabric
model, the HTTP layer is trivial:

- REST GET routes → read from Substrate.
- REST POST routes → publish a Pulse or graduate an Engram.
- WebSocket/SSE streams → forward Bus subscriptions over HTTP.

`roko-serve` becomes mostly a thin Bus-and-Substrate projection to
HTTP. Auth, schema, and rate-limiting live in the serve layer; the
data model is the same as everywhere else in the system.

The agent sidecar in `roko-agent-server` works the same way: each
running agent has its own Bus (or a namespaced topic prefix on the
shared Bus) and exposes it via WebSocket. That's how the TUI's
F3 Agents tab renders live streams.

## 5. Mesh — inter-agent coordination — Phase 2+

`docs/00-architecture/18-tools.md` and `docs/14-identity-economy`
reference a future agent mesh for peer-to-peer relay and
permissioned subnets. The naming glossary at `01-naming-and-glossary.md`
calls it "Agent Mesh / Mesh" (formerly Styx).

With two fabrics, mesh is trivially:

- **MeshBus** — a Bus backend that fans out Pulses over NATS or a
  libp2p gossipsub topology. Agents subscribe to topics they care
  about.
- **MeshSubstrate** — a Substrate backend that replicates Engrams
  over the same transport. Could be CRDT-based; could use the
  chain as arbiter.

No part of the core architecture changes when the mesh crate lands.
It's a backend swap.

## 6. Multi-agent collectives — Phase 5+

`docs/00-architecture/14-c-factor-collective-intelligence.md`
describes collective intelligence metrics — how *groups* of agents
perform, not just individuals.

Collectives in the two-fabric model are pub/sub topologies:

- A **Swarm** is N agents subscribed to the same topic set; each
  publishes its own findings; the collective outcome is the union
  of all Pulses and Engrams.
- A **Pipeline** is a chain of topic subscriptions — agent A
  publishes to `work.stage1.done`, agent B subscribes, publishes
  `work.stage2.done`, etc.
- A **Committee** is a fan-in topology — N agents publish votes to
  `decision.vote`; a single aggregator publishes the result to
  `decision.result`.

All three are just topic wiring. No orchestrator code changes.
This is what the "generalized, modular, flexible, extensible"
language in the user's request actually cashes out to.

## 7. Heartbeat (Phase 16)

`docs/00-architecture/16-heartbeat.md` describes a "cognitive clock"
that runs at three speeds (Gamma/Theta/Delta). Today it doesn't
exist; in the two-fabric model it's a `HeartbeatPolicy` that
publishes `heartbeat.gamma.tick`, `heartbeat.theta.tick`, and
`heartbeat.delta.tick` Pulses on schedule. Every speed-adaptive
subsystem subscribes to the appropriate topic. The clock itself is
fifty lines.

## 8. Safety / Provenance (Phase 11)

Safety's audit model already assumes content-addressed Engrams for
the long-term forensic DAG. Two-fabric doesn't change that — it
adds live *detection* of violations. A `SafetyPolicy` subscribes to
`tool.call.started` Pulses, checks the intended op against role
permissions, and publishes `safety.approval.requested` or
`safety.violation.detected` Pulses as appropriate. The Engram DAG
preserves the whole trail.

## 9. Daimon (affect engine, Phase 9) — cross-cut

`docs/00-architecture/09-daimon.md` describes PAD-vector affect as a
cross-cut injected across layers. Daimon currently updates its PAD
vector by being called from `orchestrate.rs` after gate verdicts.
In the two-fabric model Daimon subscribes to `gate.verdict.emitted`
and `agent.turn.completed` Pulses directly, updating PAD without any
orchestrator wiring. Consumers of PAD (the CascadeRouter, the
Composer's affect-biased scoring) read it from a Daimon
trait-object injection as today. Decoupled but cross-cut —
exactly what the cross-cut concept is supposed to be.

## 10. Dreams / Neuro cross-pollination

A specific Phase-2+ win: when Dreams produces consolidated
insights, it publishes `neuro.insight.promoted` Pulses. Neuro's
tier-progression policy subscribes and moves Engrams between
tiers (Transient → Working → Semantic → Procedural). The
orchestrator's context-enrichment path subscribes and rebuilds its
enrichment cache. All of this is reactive in two-fabric; in
one-noun it would require polling and is why doc 24 marked all
those arrows as MISSING.

## 11. Summary

| Phase | Subsystem | One-noun pain | Two-fabric resolution |
|---|---|---|---|
| 6 | Chain | ChainSubstrate conflates storage and events | Split into ChainSubstrate + ChainBus |
| 5C | Dreams | Polling Substrate | Subscribe to `substrate.engram.stored` |
| 13 | Coordination / Stigmergy | Custom pheromone plumbing | Pheromone = Engram in shared Substrate + `mesh.pheromone.*` Pulse |
| 12 | HTTP serve | Ad-hoc stream conversion | Bus projection over HTTP |
| 2+ | Mesh | Requires new trait family | Just another Bus/Substrate backend |
| 5+ | Multi-agent collectives | Requires bespoke orchestration | Pub/sub topologies |
| 16 | Heartbeat | Separate clock mechanism | `HeartbeatPolicy` on the Bus |
| 11 | Safety | Audit-only, no live detection | Subscribe to tool-call Pulses |
| 9 | Daimon | Explicit orchestrator wiring | Subscribe to verdict + turn Pulses |

Every cell in the right column is simpler than its left-column
counterpart. The two-fabric refactor pays for itself at the L0
kernel level *and* at every Phase-2+ extension point.

## 12. Why the user's original framing was right

> "it is structure for the whole stack, down to the lowest level,
> and should be generalized enough that you can create agents in
> Rust, and run them however you want, compose them, extend them,
> etc — there's lots of data that flows through eventbusses, and
> things are generalized, modular, flexible, and extensible enough
> to be able to be performant, smart, and create intelligence."

This description doesn't fit the one-noun framing: "eventbusses"
plural, composed agents, extensibility at every layer. It fits the
two-fabric / two-medium framing exactly:

- **Two fabrics** give you the eventbusses *and* the persistent
  store as peer primitives.
- **Six operators** give you the algebra to build agents by
  composition rather than inheritance.
- **Five layers** give you the structural skeleton "down to the
  lowest level."
- **Three speeds** let performance characteristics differ without
  forking the architecture.
- **Three cross-cuts** inject intelligence (Neuro / Daimon /
  Dreams) across all of it without breaking the layer rule.

The phrase Roko should lead with isn't "one noun, six verbs." It's
closer to: **"Roko is a cognitive runtime — two mediums flowing
through two fabrics, six composable operators acting on them, five
layers of strictly-downward dependency, three adaptive speeds, three
injected cross-cuts."** That sentence carries the whole architecture
and tells the truth about the shape.

## 13. Worked scenario: a pheromone trail that follows a new bug

To make the Phase-2 payoff concrete, trace a pheromone through the
two-fabric model end-to-end. The scenario: one agent discovers a
subtle race condition; we want later agents working on the same file
to find it.

1. **Agent A** (coder, working on `src/net/client.rs`) hits a flaky
   test. The agent decides the failure is a race condition.
2. Agent A authors an Engram of kind `Pheromone` with body
   `"race condition suspected in retry loop around line 142"` and
   tags `{ file: "src/net/client.rs", function: "with_retry" }`.
   The Engram's fingerprint (HDC, see 11) encodes the tags.
3. Agent A calls `substrate.put(engram)`. Substrate stores it with
   `Decay::Ebbinghaus` (fades if never reinforced) and publishes
   `Pulse { topic: "mesh.pheromone.deposited", kind: Pheromone,
   body: ... , lineage_hint: Some(engram_hash) }` to the Bus.
4. Three hours later, **Agent B** starts a task on the same file.
   Agent B's Composer runs `substrate.query_similar(file_fingerprint,
   kind=Pheromone)` and finds Agent A's note. It gets injected into
   the prompt.
5. Agent B's first action reinforces the pheromone (demurrage §2
   `ReinforceKind::AgentQuoted`). The Engram's balance goes up; it
   persists past its natural decay.
6. Agent B proposes a fix. Gate pipeline verifies; `GateVerdict`
   lands; `gate.verdict.emitted` Pulse fires.
7. The `FixProvenancePolicy` observes the verdict, checks which
   Engrams were cited in the input context (Agent A's pheromone
   was), and publishes `pheromone.successful` with lineage pointing
   at Agent A's original. Agent A's pheromone is now a validated
   heuristic candidate (see 14).

Every step is a Substrate put or a Bus publish. There is no
"pheromone subsystem." The entire ant-colony-optimization dynamic
falls out of the kernel primitives. And because every step has
lineage, the audit trail for *why* the fix was proposed is
inspectable end-to-end — the forensic capability from
`docs/00-architecture/02-engram-data-type.md` gets the social
coordination story for free.

## 14. Timing of Phase-2 unlocks

The Phase-2 subsystems don't all land together. Rough sequencing,
with dependencies:

| Subsystem | Depends on | Relative priority | Why the order |
|---|---|---|---|
| Heartbeat clock | Phase C done | Immediate | Under 50 lines; unlocks three-speed consumers |
| Safety-live (subscribe to tool-call Pulses) | Phase C done | High | Closes P0 safety gap |
| Dreams (subscribe to `substrate.engram.stored`) | Phase C + demurrage (12) | Medium | Needs stable Engram lineage |
| Stigmergy (pheromone) | HDC-on-Engram (11) | Medium | Depends on `query_similar` |
| Mesh (NatsBus) | `roko-mesh` crate scaffold | Medium | Depends on new crate |
| ChainBus | `roko-chain` integration | Low | Requires on-chain attestation story |
| Collectives (pub/sub topologies) | Mesh done | Low | Patterns on top of mesh |
| Daimon (live PAD) | Phase C done | Low | Scoped by team capacity |

The refactor from 01–08 is a precondition for most of these, which
is why the foundation work is worth doing before Phase-2 builds on
top. `35-consolidated-roadmap.md` stitches this list into the full
multi-quarter plan.

--- END 09-phase-2-implications.md ---

# Batch REF09 — Phase-2 implications: chain / dreams / mesh / coordination

**Refinement source**: `tmp/refinements/09-phase-2-implications.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- `docs/08-chain/` — introduce ChainBus vs ChainSubstrate split.
- `docs/10-dreams/` — document Substrate scan + Bus-subscription input.
- `docs/13-coordination/` — stigmergy as pheromone Engram + mesh.pheromone Pulse.
- `docs/16-heartbeat/` — HeartbeatPolicy publishes heartbeat.{gamma,theta,delta}.tick Pulses.
- `docs/00-architecture/24-cross-section-integration-map.md` — Bus-based integration supersedes prior proposals.

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/09-phase-2-implications.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.
- Each touched subdir's INDEX.md should summarize the new framing briefly.

## Required vocabulary (verify)

The verify step greps for: `ChainBus|two.?fabric`

At least one changed file must mention it (case-insensitive) before the
batch is considered complete.

## Cross-references

Dependents — downstream refinements assuming this one has propagated
into docs: REF32, REF33

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
- Commit ready with message `refinements(REF09): Phase-2 implications: chain / dreams / mesh / coordination`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
