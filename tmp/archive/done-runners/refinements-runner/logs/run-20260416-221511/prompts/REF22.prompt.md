# Refinements Batch REF22

Run id: run-20260416-221511
Attempt: 1
Model: gpt-5.4
Reasoning: high
Refinement source: /Users/will/dev/nunchi/roko/roko/tmp/refinements/22-developer-ux-rust.md
Target docs (candidates): docs/12-interfaces/ docs/02-agents/ docs/00-architecture/INDEX.md

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

Suggested parallel split for batch `REF22`:

- worker: add/update files under `docs/12-interfaces/` describing the
  four-layer Rust SDK (one-liner / builder / trait / runtime).
- worker: update `docs/02-agents/` where custom-agent authoring is discussed.

## Canonical refinement source

This is the verbatim refinement proposal that this batch must propagate
into `docs/`. Treat it as the authoritative source. Do not edit this
file; only edit docs under `docs/`.

--- BEGIN 22-developer-ux-rust.md ---

# Developer UX: Building Agents in Rust

> **TL;DR**: The single largest obstacle to Roko's adoption by Rust
> developers is not performance, correctness, or features — it's
> *time to first working agent*. Today that number is measured in
> hours. This doc proposes a layered Rust SDK with four ergonomic
> entry points (one-liner, builder, trait-impl, runtime-impl), a
> deliberate error vocabulary, a plugin-aware examples/ directory,
> and documentation discipline that turns Roko into something a
> Rust dev can reach for the same way they reach for tokio. Target
> metric: "hello, agent" in under 60 seconds for any Rust dev who
> has cargo installed.

### For first-time readers

A handful of terms recur throughout this doc. Brief definitions:

- **Engram** — durable, BLAKE3-addressed record (episode, heuristic,
  plan, claim). Lives in a `Substrate`. See `02-engram-vs-pulse.md`.
- **Pulse** — ephemeral, sequenced in-flight message on the `Bus`.
  See `03-bus-as-first-class.md`.
- **Substrate / Bus** — the two kernel fabrics: durable store vs
  ephemeral stream. Both are traits a developer can implement.
- **Operator** — one of Scorer / Gate / Router / Composer / Policy.
  Operators consume either medium and are the units of extension.

## 1. The audiences

Four developer audiences, each with a different relationship to the
kernel. The SDK needs a pleasant surface for each.

1. **Application author**: wants to embed an agent in their Rust
   program. Doesn't want to know about Substrate traits.
2. **Agent author**: wants to build a role-specific agent with
   custom tools, templates, and maybe a gate. Needs the builder
   surface.
3. **Trait implementor**: wants to swap out a Substrate, Bus,
   Scorer, or Router with their own. Needs stable trait contracts.
4. **Runtime implementor**: wants to build a new execution mode
   (e.g., browser-based, edge, distributed). Needs access to the
   kernel types directly.

The four layers should be *visually* different. A one-line example
shouldn't look like a trait impl. A trait impl shouldn't require
writing boilerplate to reach the kernel.

## 2. The four entry points

### 2.1 One-liner (for demos and scripts)

```rust
use roko::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let out = roko::run("Summarize README.md").await?;
    println!("{out}");
    Ok(())
}
```

`roko::run` should pick sensible defaults: local model if available,
Claude if not, memory in `.roko/`, no plugins. This is the
"anybody can paste this in and see it work" hook.

### 2.2 Builder (for configured agents)

```rust
use roko::{Agent, Role, Model};

#[tokio::main]
async fn main() -> Result<()> {
    let agent = Agent::builder()
        .role(Role::Researcher)
        .model(Model::claude_opus())
        .tool(tools::web_search())
        .tool(tools::fs_read("."))
        .memory_dir("./.roko")
        .build().await?;

    let response = agent.send("What's new in Rust 1.91?").await?;
    println!("{response}");
    Ok(())
}
```

The builder is the *daily driver*. 90% of application authors
never leave this layer. It should:

- Return typed errors, not strings.
- Fail at `.build()` for misconfiguration, not at `.send()`.
- Have a `dry_run()` that prints the system prompt it would use.
- Support `clone()` for spawning worker agents from a template.

### 2.3 Trait impl (for custom kernel parts)

```rust
use roko_core::{Substrate, Engram, EngramHash, Result};

pub struct MySubstrate { /* ... */ }

#[async_trait]
impl Substrate for MySubstrate {
    async fn put(&self, e: Engram) -> Result<EngramHash> { ... }
    async fn get(&self, h: &EngramHash) -> Result<Option<Engram>> { ... }
    async fn query(&self, p: Predicate) -> Result<Vec<Engram>> { ... }
    async fn scan(&self, range: HashRange) -> Result<Stream<Engram>> { ... }
    async fn freeze(&self, h: &EngramHash) -> Result<()> { ... }
    async fn thaw(&self, h: &EngramHash) -> Result<Engram> { ... }
}

// Use it:
let agent = Agent::builder()
    .substrate(MySubstrate::new(...))
    .build().await?;
```

Trait impls should be *self-contained*: the trait should require
only what's in `roko-core`. No type leakage from the runtime tier
into the trait contract.

### 2.4 Runtime impl (for new execution modes)

```rust
use roko_runtime::{Runtime, Supervisor, EventBus};

pub struct BrowserRuntime { /* WASM-specific state */ }

impl Runtime for BrowserRuntime {
    type Supervisor = BrowserSupervisor;
    type Bus = InMemoryBus;
    fn supervisor(&self) -> &BrowserSupervisor { &self.supervisor }
    fn bus(&self) -> &InMemoryBus { &self.bus }
}

// ...
let agent = Agent::builder()
    .runtime(BrowserRuntime::new())
    .build().await?;
```

This is rarely used but *must exist* — otherwise Roko is locked to
tokio-on-Linux and can't target browsers, edge, or embedded.

## 3. Error vocabulary

Errors are UX. Rust makes them visible; Roko should make them
*useful*. The canonical shape:

```rust
#[derive(Debug, thiserror::Error)]
pub enum RokoError {
    #[error("substrate error: {0}")]
    Substrate(#[from] SubstrateError),

    #[error("bus error: {0}")]
    Bus(#[from] BusError),

    #[error("tool {tool} failed: {reason}")]
    Tool { tool: String, reason: String },

    #[error("gate {gate} rejected: {reason}")]
    Gate { gate: String, reason: String },

    #[error("agent timed out after {}ms", .0.as_millis())]
    Timeout(Duration),

    #[error("configuration invalid: {0}")]
    Config(String),

    // ... more
}
```

Rules:

- **Never** bubble up `anyhow::Error` at the public API surface.
- **Every** error variant must be actionable (tell the user what
  to try).
- **`#[non_exhaustive]`** on every enum so adding variants is
  non-breaking.
- **Backtrace** attached in debug builds; elided in release.

## 4. Docs discipline

Four levels of documentation, each targeting a different audience:

### 4.1 `README.md` (for discovery)

60-second pitch. Copy-pasteable one-liner. Link to docs site.

### 4.2 `docs/tutorials/` (for first-hour users)

Guided walkthroughs: "Build a research agent in 10 minutes,"
"Add a custom tool," "Swap the Substrate." Linear, with every
code block runnable as-is.

### 4.3 `docs/cookbook/` (for returning users)

Recipe format: "I want to do X, show me the minimum code."
Searchable by goal, not by API shape. Examples:

- "Run an agent against a codebase"
- "Stream responses to a web UI"
- "Swap memory backends"
- "Add a compliance gate"

### 4.4 `docs/reference/` (for API hunters)

Auto-generated from rustdoc, but curated: hand-written overviews
per crate explaining its role in the larger architecture.

### 4.5 Rustdoc on every public item

Not optional. Every `pub fn`, `pub struct`, `pub trait` has:

- One-line summary.
- Example code block.
- Cross-link to related items.
- `# Errors` section if it returns Result.
- `# Panics` section if it can panic.

Enforce with `#![warn(missing_docs)]` in each crate.

## 5. The `examples/` directory

Worth taking seriously. Proposed structure:

```
examples/
├── 01-hello-agent/         — the one-liner
├── 02-builder-basics/      — builder with one tool
├── 03-custom-tool/         — implementing a tool
├── 04-custom-gate/         — implementing a gate
├── 05-swap-substrate/      — using a custom Substrate
├── 06-multi-agent/         — coordinating two agents
├── 07-streaming/           — streaming responses via Bus
├── 08-plugin-manifest/     — declarative tool plugin
├── 09-research-workflow/   — paper → claim → heuristic
├── 10-web-ui/              — serving to a minimal web UI
```

Every example is a working cargo project. Every example's README
links to the relevant docs. `cargo test --examples` runs them all
in CI.

## 6. Builder patterns Rust developers already know

Borrow from the best:

- **`tokio::runtime::Builder`**: explicit build step, typed errors,
  sensible defaults.
- **`reqwest::ClientBuilder`**: fluent methods, no-magic semantics.
- **`serde::Serializer` trait**: narrow, stable, implementable.
- **`sqlx`**: compile-time query validation, derive macros for
  types.
- **`bevy_ecs`**: plugin pattern via App::add_plugin(...).

Roko's APIs should feel like they belong in this family. The
gesture is consistent fluent `.x().y().build()` with typed output.

## 7. Macros where they earn their keep

Macros are UX but abused are anti-UX. Proposed macros:

### 7.1 `#[tool]`

```rust
#[tool(
    description = "Read a file from disk",
    role_allow = ["researcher", "implementer"],
    files = "{path}",
)]
async fn read_file(path: String) -> Result<String> {
    tokio::fs::read_to_string(&path).await.map_err(Into::into)
}
```

Expands to a `Tool` impl with schema derived from the signature.

### 7.2 `#[gate]`

```rust
#[gate(rung = Rung::Unit)]
async fn my_gate(ctx: &GateCtx) -> GateResult { ... }
```

### 7.3 `claim!`

From `16-research-to-runtime.md` — resolves a Claim at build time
and produces a tracked parameter at runtime.

### 7.4 Prompt template DSL

```rust
let prompt = prompt! {
    role: Role::Researcher,
    context: episode.recent(5),
    situation: current_task,
    heuristics: hdc_match(0.7),
};
```

A declarative prompt builder that the Composer compiles. Beats
string concatenation.

## 8. `cargo roko` — dev workflow subcommand

A cargo plugin for agent dev workflow:

```bash
cargo roko new my-agent           # scaffold a new agent project
cargo roko play                    # REPL for iterating on prompts
cargo roko replay <episode>        # re-run an episode with new code
cargo roko bench                   # benchmark against a task suite
cargo roko explain <hash>          # trace a decision through operators
cargo roko heuristics              # browse current beliefs
```

These are the Rust-dev versions of `roko` CLI commands, scoped to
the current crate. They compose with existing cargo workflows and
make Roko feel like a first-class cargo citizen.

## 9. Type signatures as documentation

Rust's type system is expressive enough that *signatures alone*
can carry intent. Some opinions:

### 9.1 `Engram` vs `&Engram` vs `EngramHash`

- Take `&Engram` when you read.
- Take `Engram` when you'll mutate and return.
- Take `EngramHash` when you want to defer resolution.

Don't mix these casually. The right choice saves allocations and
tells the reader what will happen.

### 9.2 `async fn` vs `Stream`

- `async fn -> T` for single-shot operations.
- `async fn -> Stream<T>` for fan-out.
- Never `async fn` that returns a future of a stream of futures —
  flatten it.

### 9.3 `Box<dyn Trait>` vs generic parameter

- Generic `T: Trait` when the type is known at composition time.
- `Box<dyn Trait>` only when heterogeneity is required (e.g.,
  plugin dispatch).

### 9.4 `Result` typing

- `Result<T>` with crate's `Error` type at top of the module.
- Full type `Result<T, SpecificError>` only at crate boundaries.

## 10. Testing ergonomics

Agent testing is hard. Roko should ship testing helpers.

```rust
use roko::testing::{MockAgent, AssertingBus};

#[tokio::test]
async fn my_flow_hits_gate() {
    let agent = MockAgent::builder()
        .expect_tool("read_file", "fake contents")
        .build();
    let bus = AssertingBus::expect("gate.passed.unit");

    agent.run(bus, "read README.md").await.unwrap();

    bus.assert_expectations();
}
```

- **MockAgent**: scripted responses without hitting a model.
- **AssertingBus**: declarative expectations on Bus activity.
- **RecordingSubstrate**: captures all writes for snapshot testing.
- **TimeTravel**: advance the `demurrage` clock explicitly to test
  decay behavior.

All under `roko::testing::*`. None in `#[cfg(test)]`-only code —
these are for downstream tests too.

### 10.1 A second, end-to-end example

A heuristic-sensitive test that wires StateHub projections (see
`26-statehub-rearchitecture.md` §3) into the harness:

```rust
use roko::testing::{AgentHarness, RecordingSubstrate, FakeClock};
use roko::heuristics::HeuristicId;

#[tokio::test]
async fn retries_after_flaky_test_heuristic_fires() {
    let clock     = FakeClock::at("2026-04-16T09:00:00Z");
    let substrate = RecordingSubstrate::new();
    let mut h     = AgentHarness::new()
        .with_clock(clock.clone())
        .with_substrate(substrate.clone())
        .seed_heuristic(HeuristicId::from("flaky-test-log-first"))
        .build().await.unwrap();

    h.enqueue("run the failing test and diagnose").await;
    h.tick_until_idle().await;

    assert!(substrate.writes_matching("episode.tool.read_file").len() >= 1);
    assert!(h.projection::<GateHistory>().any(|g| g.rung == Rung::Unit));
    assert_eq!(h.applied_heuristics(), ["flaky-test-log-first"]);
}
```

### 10.2 `cargo test` and `cargo bench` integration

Roko ships first-class cargo integration so tests and benches don't
need a special runner:

- **`roko::testing::harness_main!()`** — expands at the top of a
  test file; rewires the tokio runtime, installs a `FakeClock`, and
  enables `#[roko_test]` attributes with fixtures.
- **`criterion` adapters** — `roko::bench::agent_bench(b, |h| ...)`
  returns a `Criterion` benchmark over a pre-seeded `AgentHarness`
  so you can measure plan latency, compose time, and gate cost as
  normal Criterion benches.
- **`cargo roko bench`** (see §8) wraps `cargo bench` and emits a
  run record into `.roko/learn/efficiency.jsonl` for regression
  tracking across commits.
- **Snapshot tests** — `RecordingSubstrate::assert_snapshot("name")`
  writes under `tests/snapshots/`; updates via `UPDATE_SNAPSHOTS=1`.

Net effect: the same `cargo test`/`cargo bench` muscle memory works,
but each test gets a reproducible substrate + bus + clock.

## 11. Release cadence and compatibility

Developers need *predictable* API evolution. Proposed rules:

- **SemVer strict**: major-bump for any trait signature change in
  `roko-core`, `roko-spi`, `roko-bus`, `roko-hdc`.
- **6-week release train**: minor versions every 6 weeks, patch
  versions on demand.
- **MSRV policy**: always stable Rust, upgrade MSRV only in minor
  releases, never in patch.
- **CHANGELOG.md**: every PR updates it. Keep-a-changelog format.
- **Deprecation**: two minor versions of deprecation before removal,
  with a migration guide entry.

This is mundane and load-bearing. Projects that skip it bleed
developers.

## 11.5 Debug builds, logging, tracing

Observability for a developer's own custom agents. The same
spans/metrics that `24-deployment-ux.md` §5 exposes in production
should be available in local dev, with less ceremony.

- **`tracing` everywhere**: `roko-core` emits spans at operator
  boundaries. Downstream code inherits them — `tracing::instrument`
  on a custom `Scorer` nests correctly under the kernel span.
- **`roko::log::init()`** — one call at `main` wires `tracing_subscriber`
  with sensible filters (`info` for your crate, `warn` for deps),
  respects `RUST_LOG`, and routes structured logs to stderr JSON
  when `--format json` is set on the runtime.
- **Span conventions**: every operator emits a span named
  `op.<kind>` with fields `{operator_id, pulse_seq?, engram_hash?}`
  so traces correlate cleanly to `27-realtime-event-surface.md`
  cursors.
- **Pulse inspector**: `roko::dev::tap_bus()` returns a
  `tracing::Subscriber` that also forwards every Pulse as a
  structured log — useful when your custom `Composer` appears to
  drop events.
- **`#[roko::instrument]`** — macro sugar over `tracing::instrument`
  that adds `operator_kind`, `agent_id`, and an auto-generated
  correlation id so multi-agent flows stay readable.
- **Debug-build assertions**: `debug_assertions` enables extra
  invariants in `roko-runtime` (e.g. Bus back-pressure never
  silently drops, Substrate writes always round-trip). Release
  builds elide them.

A pragmatic pattern: wire a `RecordingSubstrate` + a `tracing` layer
exporting to a local file, then `cargo roko explain <hash>` (see
§8) replays the trace with operator boundaries highlighted. See
also `26-statehub-rearchitecture.md` §5 on projecting bus traffic
into a live-updating debug view.

## 12. What "world-class dev UX" actually means

A Rust developer should be able to:

1. See Roko on GitHub, read the README, believe in it — in under
   a minute.
2. Clone the repo or `cargo add roko` and get a working one-liner
   — in under 5 minutes.
3. Build a custom tool and run it — in under 30 minutes.
4. Swap a Substrate or Bus impl with their own — in under half a
   day.
5. File an issue that gets a substantive response, not a robot
   reply — within a couple of days.

All five are achievable with the practices in this doc. Most of
them are *not* achievable with the current state of the repo —
not because the code is bad but because the SDK surface and
examples/docs haven't been designed as a product. They need to be.

### Related refinements

- `23-user-ux-running-agents.md` §2 — the verb set a developer's
  custom agent inherits for free at runtime.
- `26-statehub-rearchitecture.md` §3 — typed projections the harness
  in §10 exposes to downstream tests.
- `27-realtime-event-surface.md` §4 — WebSocket/SSE wire format
  that custom `Bus` impls must speak to interop with official
  clients.
- `17-plugin-extension-architecture.md` §2 — tier boundaries that
  govern where a given extension (macro-generated tool, native
  crate, WASM module) lives.
- `25-domain-specific-agents.md` §9 — how a developer packages their
  tools, heuristics, and gates as an installable profile bundle.

--- END 22-developer-ux-rust.md ---

# Batch REF22 — Developer UX: four-layer Rust SDK across interfaces

**Refinement source**: `tmp/refinements/22-developer-ux-rust.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- `docs/12-interfaces/` — four-layer Rust SDK (one-liner / builder / trait / runtime) chapter(s).
- `docs/02-agents/` — custom-agent authoring references.
- `docs/00-architecture/INDEX.md` — link dev-SDK chapter.

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/22-developer-ux-rust.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.


## Required vocabulary (verify)

The verify step greps for: `one.?liner|builder|trait.?impl|runtime.?impl|SDK`

At least one changed file must mention it (case-insensitive) before the
batch is considered complete.

## Cross-references

Dependents — downstream refinements assuming this one has propagated
into docs: REF25

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
- Commit ready with message `refinements(REF22): Developer UX: four-layer Rust SDK across interfaces`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
