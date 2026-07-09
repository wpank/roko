# Refinements Batch REF23

Run id: run-20260416-221511
Attempt: 1
Model: gpt-5.4
Reasoning: high
Refinement source: /Users/will/dev/nunchi/roko/roko/tmp/refinements/23-user-ux-running-agents.md
Target docs (candidates): docs/12-interfaces/ docs/00-architecture/INDEX.md

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

Suggested parallel split for batch `REF23`:

- worker: add/update files under `docs/12-interfaces/` for the unified
  verb set, four surfaces (CLI/TUI/Chat/Web), first-run flow, undo model.

## Canonical refinement source

This is the verbatim refinement proposal that this batch must propagate
into `docs/`. Treat it as the authoritative source. Do not edit this
file; only edit docs under `docs/`.

--- BEGIN 23-user-ux-running-agents.md ---

# User UX: Running Agents

> **TL;DR**: Non-developer users (and developers acting as users)
> interact with Roko through four surfaces: CLI, TUI, Chat, and
> Web. Today CLI is dominant; TUI is wired; Chat exists; Web is
> via the HTTP control plane. This doc proposes a unified
> interaction model where all four surfaces expose the *same*
> underlying verbs over the *same* event stream, so muscle memory
> transfers freely. It also proposes "familiar-first" defaults
> modeled on Claude Code, where users get productive without
> reading docs. The goal: a user who types `roko` for the first
> time produces useful output in under 30 seconds.

### For first-time readers

Key terms used below:

- **Bus** — ephemeral message stream inside Roko (tool banners,
  token streams, gate results). See `03-bus-as-first-class.md`.
- **Engram** — durable record: an episode, plan, heuristic, PRD.
  See `02-engram-vs-pulse.md`.
- **Surface** — one of CLI / TUI / Chat / Web. Each one renders the
  same verbs over the same Bus/Engram data.
- **Heuristic** — a named, calibrated belief the agent applies. See
  `14-worldview-validation.md`.

## 1. The four surfaces

| Surface | Today | Gaps |
|---|---|---|
| **CLI** | `roko run`, `plan`, `prd`, etc. | Inconsistent flag names, some subcommands missing status output |
| **TUI** | `roko dashboard`, F1–F7 tabs | Read-mostly; needs more interactive actions |
| **Chat** | `roko chat --agent <id>` | Only routes to a single agent; no multi-agent or streaming polish |
| **Web** | `roko serve` exposes ~85 routes | No first-party UI — users must build their own |

The surfaces overlap conceptually but diverge in flag names and
defaults. A user who knows `roko run "..."` has no idea what the
equivalent is in the TUI.

## 2. One verb set, four renderings

Propose a canonical verb set exposed by every surface:

| Verb | Meaning |
|---|---|
| **ask** | Run a single-turn query |
| **plan** | Propose a plan without executing |
| **do** | Execute a plan or a single task |
| **watch** | Stream progress of active work |
| **inspect** | Drill into an episode, engram, or heuristic |
| **replay** | Re-run a prior episode, optionally with changes |
| **learn** | Browse / curate heuristics, playbooks, experiments |
| **tune** | Change configuration, thresholds, routing |
| **connect** | Add a plugin, MCP, credential |

Every surface has a rendering of each verb. The user learns the
verbs once; the interface is wherever they happen to be.

## 3. First-run experience

The current first-run experience:

```bash
$ cargo install roko-cli      # 5 minutes compiling
$ roko init                   # creates .roko/
$ roko run "..."              # works if you configured a model
```

Gaps: users who haven't set up Claude/OpenAI API keys get cryptic
errors. No guided onboarding. No "plugin check." No heuristic-commons
opt-in dialog.

Proposed replacement: `roko init` becomes interactive.

```
$ roko init
Welcome to Roko. Let's get you set up.

Which models would you like to use?
  [x] Claude (requires ANTHROPIC_API_KEY)
  [ ] OpenAI (requires OPENAI_API_KEY)
  [ ] Local Ollama (detected at http://localhost:11434)
  [ ] Codex / Cursor / Gemini / Perplexity

Where should your agent memory live?
  > ./.roko  (recommended; git-ignored automatically)

Would you like to start with a heuristic library?
  [x] Import the starter kit (20 calibrated heuristics)
  [ ] Start from scratch

Should Roko look for existing MCP servers?
  [x] Yes, auto-discover
  [ ] No, manual config only

Done. Try:
  roko ask "what's my first task?"
  roko dashboard
  roko plugin list

Docs: https://roko.dev/docs/getting-started
```

This is a 30-second interaction that sets a user up for success.
Current init is non-interactive and assumes knowledge the user
may not have.

### 3.1 Error recovery inside `roko init`

The init flow has to degrade gracefully. Expected failure modes and
recoveries:

```
$ roko init
...
Which models would you like to use?
  [x] Claude (requires ANTHROPIC_API_KEY)

Checking ANTHROPIC_API_KEY... not found in env.
  1) Paste a key now (stored in OS keychain)
  2) Open https://console.anthropic.com/account/keys
  3) Skip Claude, try another provider
  4) Configure later with `roko secret set anthropic.api_key`
  >
```

```
Detecting local models... Ollama not reachable at :11434.
  1) Start Ollama and retry
  2) Skip — continue without local fallback
  3) Install Ollama (opens docs)
  >
```

```
Auto-discovering MCP servers... found 2, 1 returned errors.
  - ok: code-intel (roko-mcp-code)
  - err: github-mcp (connection refused)

  r) Retry
  s) Skip errored servers
  d) Show diagnostic for github-mcp
  >
```

Rules for every prompt:

- Always offer a `skip / configure later` option; `roko init`
  should never be a dead end.
- Every failure carries a literal next command the user can run
  (`roko secret set ...`, `roko plugin doctor <id>`).
- Partial success is a first-class outcome: `roko.toml` is
  committed incrementally as each step succeeds, so a user who
  Ctrl-C's mid-flow resumes from where they stopped.
- Tenth-percentile networks: every remote check has a 5s timeout
  and a retry prompt; no step blocks for longer than that without
  a visible cancel.

See also `24-deployment-ux.md` §3 for how the same keys migrate
into server-shape secret stores later, and `28-cli-parity-familiar-workflows.md`
§3 on slash-command parity for these prompts inside chat/TUI.

## 4. CLI: consistency, conventions, colors

### 4.1 Flag conventions

Adopt a house style. Every subcommand:

- `--format {human|json|yaml}` with `human` default.
- `--quiet` and `--verbose` with predictable volume.
- `--no-color` respects `NO_COLOR` env.
- `--dry-run` wherever side effects happen.
- `--plan-file <path>` never positional; positional slots are
  reserved for subjects (the plan ID, the hash, the prompt).

### 4.2 Output shape

- Success paths: single-line summary → details behind `--verbose`.
- Errors: include a *"try:"* line with a remediation hint.
- Long-running: progress via carriage-return overwrite in TTY,
  one-line-per-event in pipes.
- Colors are semantic (green=success, red=error, yellow=warning),
  not decorative.

### 4.3 Help that teaches

```bash
$ roko help ask
USAGE: roko ask <prompt> [options]

Ask the agent to respond to a single prompt.

OPTIONS:
  --role <role>          Which role to use (researcher, implementer, ...)
  --model <model>        Override the model routing
  --stream               Stream the response as it's produced
  --save                 Persist to an episode (default: ephemeral)
  --context <path>       Include a file or directory as context

EXAMPLES:
  roko ask "what does this codebase do?"
  roko ask "fix the failing test" --role implementer --stream
  roko ask "summarize" --context README.md

RELATED:
  roko plan     Propose a plan
  roko do       Execute a plan
  roko watch    Watch an in-flight conversation
```

Every help page teaches the *next step*. RELATED lines are
load-bearing for discovery.

## 5. TUI: interactive, not just displays

Today's TUI is mostly read-only. The seven F-tabs display
episodes, plans, gates, etc. A modern TUI would let the user *act*.

Proposed interactions added to F-tabs:

- **Episodes tab**: `r` to replay a selected episode; `i` to inspect
  its heuristic citations; `/` to search.
- **Plans tab**: `x` to execute the selected plan; `e` to edit the
  plan's markdown; `p` to pause/resume.
- **Gates tab**: `t` to adjust thresholds interactively; `v` to
  view recent failures.
- **Heuristics tab** (new): `c` to challenge a heuristic (force
  recalibration); `r` to retire; `e` to edit.
- **C-factor tab** (new): visualizations with keyboard zoom.
- **Chat tab**: full-duplex chat with any running agent.

The TUI becomes a control surface, not just a dashboard. Every
action the CLI can do should have a TUI binding.

## 6. Chat: streaming, multi-agent, inline artifacts

Chat today is one-agent-at-a-time. Upgrades:

- **Multi-agent chat**: `@researcher` and `@implementer` as address
  prefixes route to different roles; you can see both responding.
- **Streaming with cursor**: response tokens appear live.
- **Inline artifacts**: when an agent produces a code block, the
  chat renders it as a fenced block with *apply*, *copy*, and
  *diff* affordances, not just text.
- **Slash commands**: `/plan`, `/run`, `/explain`, `/heuristics`,
  `/replay` available from within chat so you don't have to leave
  to take actions.
- **Attachments**: drop a file path onto the chat; it becomes
  context for the next turn.

Feels like Claude Desktop or Claude Code but native to Roko's
runtime and with access to Bus, Substrate, Heuristics.

## 7. Web: first-party UI, eventually

`roko serve` exposes the API. A first-party web UI shipped with it
(or in a sibling repo) would give non-CLI users a way in. Initial
scope:

- **Home**: current state of the agent runtime (what's running,
  how long, c-factor).
- **Ask**: a chat interface rendering in real-time.
- **Plans**: tree view of plans and tasks with DAG visualization.
- **Episodes**: searchable list with replay.
- **Heuristics**: the externalized-beliefs browser.
- **Settings**: visual configuration.

Must be a small number of pages — not a kitchen sink. Web UI
targeting completeness is a maintenance trap; targeting
discoverability is sustainable.

## 7.5 Power-user shortcuts

Once a user has done the basics three times, they're a power user
and every extra keystroke is a papercut. First-class shortcuts:

- **Cmd-K / Ctrl-K command palette** — the surface-agnostic index
  of every verb and subcommand. Available in TUI, Chat, and Web.
  Fuzzy-matches against command, synonyms, recent args, and
  heuristic names. Arrow-keys navigate; Enter runs; Shift-Enter
  copies the equivalent CLI invocation to the clipboard.
- **Named sessions** — `roko session use research-q2` attaches to a
  persistent session; the TUI title-bar shows the name; chat and
  Web auto-scope to it. `roko session list` shows recent sessions
  ordered by last-touched. Sessions are Engrams too (see
  `02-engram-vs-pulse.md` §4), so they replay cleanly.
- **Recent-prompt history** — `Ctrl+R` in CLI/Chat opens fuzzy
  history (scoped to the current session by default; `Ctrl+Ctrl+R`
  widens to all sessions). History is indexed by both prompt text
  and the heuristics that fired.
- **Bookmarks** — `#` on any episode/engram hash pins it; `@#slug`
  resolves anywhere a prompt is accepted. Bookmarks sync to the
  heuristic-commons client, so the same label works across devices.
- **Keybinding profiles** — `--keymap {default,vim,emacs}` flips
  the TUI and Chat keyset. Profiles are config-editable and
  documented in one place.
- **Batch mode** — `roko ask --batch prompts.txt` fans prompts over
  a pool; output as JSONL. Pairs with `27-realtime-event-surface.md`
  cursors so a long batch can be monitored from the Web UI.

These aren't free; every shortcut has a cost in discoverability
debt. But they're what turns a user from "productive" into "fast."

Users come with expectations from Claude Code, OpenClaw, Cursor,
Aider, and chat interfaces. Adopt rather than innovate where it
costs nothing:

- **`:` to open command palette** (like VS Code).
- **`/` to slash-command** (like Claude Code / Discord).
- **`?` for context help** (like vim).
- **`q` to quit** (like less / top / vim).
- **`j/k` for navigation** (TUI-wide convention).
- **`Ctrl+R` for fuzzy history search** (bash).
- **`#` for anchoring a thought/task** (like GitHub issues).

Familiar keystrokes produce goodwill; novel keystrokes produce
frustration until they compound into muscle memory. Spend novelty
carefully.

## 9. Live progress, not polling

When an agent is working:

- **Token streaming** from the LLM.
- **Tool call banners**: "→ reading src/main.rs (line 1–200)" as
  the tool fires.
- **Gate feedback**: "✗ Unit tests failed (3 of 47)" as gates fail.
- **Episode events**: "+ heuristic 'flaky-test-log-first' applied
  (confidence 0.82)" as beliefs feed the decision.

The user feels the agent *thinking* rather than staring at a
stopped spinner. This is why `26-statehub-rearchitecture.md` and
`27-realtime-event-surface.md` matter — the Bus already produces
all this data; the surfaces need to render it.

## 10. Human-in-the-loop checkpoints

Some decisions should invoke the user. Three kinds:

### 10.1 Permission checkpoints

Before dangerous actions (deleting files, network calls to unseen
endpoints, hitting rate-limited APIs), ask. Default: ask once per
session for each class of action, remember within a session.

### 10.2 Ambiguity checkpoints

When the agent has low confidence between two paths, surface a
choice:

```
The agent is unsure how to proceed:

  1. Add a new module to src/net/ (confidence 0.51)
  2. Extend the existing client.rs (confidence 0.49)

  a) Always prefer 1
  b) Always prefer 2
  c) Ask me every time for this kind of decision
  d) Let the agent decide

  >
```

The choice itself becomes a heuristic ingredient — "this user
prefers extending over adding new modules." Over time the user is
asked less and less.

### 10.3 Review checkpoints

Before creating a PR or committing, show the diff. User can
approve, edit, or cancel. Default: always review; never blind
commit.

### 10.4 Permission model at a glance

| Action | Default | Remember within | Can auto-approve? |
|---|---|---|---|
| Read file inside project root | auto | — | n/a |
| Read file outside project root | ask | session | yes (per-path glob) |
| Write file inside project root | ask once | session | yes (per-directory) |
| Delete file (any path) | ask every time | never | no |
| Run local command (whitelisted) | auto | — | n/a |
| Run local command (unrecognized) | ask | session, per-command | yes |
| Network request to allowlisted host | auto | — | n/a |
| Network request to new host | ask | session, per-host | yes |
| Spend > `$budget` on a single turn | ask | never | no |
| `git commit` / open PR | ask | never | no |
| Install/upgrade a plugin | ask | never | no |
| Execute a tool flagged `role_allow=` miss | deny | — | no (role change required) |
| Override a failing gate | ask | per-rung, per-session | yes for non-critical rungs |
| Send a chat-reply or email on user's behalf | ask every time | never | no |

Rules that bind the table:

- "Auto" actions still emit a Pulse so the TUI banner / `watch`
  stream shows what happened — silent success is still visible
  success.
- Session-scoped remember is cleared by `roko session forget` and
  by `roko init --reset-permissions`.
- Every denial carries the literal override command (e.g.
  `roko permit network:example.com --duration 1h`).
- Prohibited actions (see `24-deployment-ux.md` §3 on secret
  handling) are never promotable to auto — they remain `ask every
  time` regardless of config.

## 11. Making undo first-class

Users need to feel safe. Three levels of undo:

- **Ephemeral**: a recently-sent prompt can be edited (chat).
- **Short-term**: `roko undo last` reverts file changes from the
  last task.
- **Long-term**: every episode has a snapshot; `roko replay
  <episode>` can replay from there or `roko revert <episode>` can
  undo its diffs.

Undo being real makes users more adventurous. More adventurous
usage → more reinforcement signal → more learning.

## 12. Shareable, replayable sessions

Every session should be exportable:

```bash
roko session export > my-session.jsonl
roko session share --expires 24h     # uploads to registry, returns URL
```

Another user (or the same user later) can `roko session replay`.
This powers bug reports ("here's my session, reproduce"), blog
posts, and tutorials. It's also the unit of empirical training
data across deployments for the heuristic commons.

## 13. Accessibility

Not optional. Non-negotiables:

- TUI colors have configurable high-contrast mode.
- Screen-reader friendly markup in Web UI.
- Every critical action also available as a CLI command (for users
  who can't / won't use graphical surfaces).
- Keyboard-only navigation in TUI and Web.
- Internationalizable strings — English by default, but no
  hardcoded strings in user-facing paths.

## 14. The moments that matter

If we get seven moments right, most users forgive everything else:

1. First successful `roko ask` (under 30 seconds).
2. First `roko do` producing a real change (under 5 minutes).
3. First time the user sees a heuristic they didn't know they
   "taught" get applied.
4. First time the user watches c-factor go up after they made a
   choice to rotate pairs / diversify / introduce a challenger.
5. First time the user recovers from a mistake via `roko undo` or
   replay.
6. First time the user switches surface mid-task (CLI → TUI,
   Chat → Web) and the session follows them without state loss.
   This is the payoff of `26-statehub-rearchitecture.md` and
   `27-realtime-event-surface.md` on the user side.
7. First time a shared session (`12` above) reproduces someone
   else's bug on the user's machine, and the commons heuristic
   the original author had kicks in locally as it did for them.

Each moment is a product-design commitment, not just a feature.
The surfaces in this doc exist to produce them reliably.

### Related refinements

- `26-statehub-rearchitecture.md` — the projections backing the
  live progress, permission banners, and multi-surface continuity.
- `27-realtime-event-surface.md` — WebSocket/SSE cursors that make
  `watch` and the Web UI feel real-time.
- `28-cli-parity-familiar-workflows.md` — the slash-command and
  palette conventions this doc leans on.
- `30-rich-ux-primitives.md` — tool banners, uncertainty bars, and
  heuristic footnotes that render the Bus stream.
- `22-developer-ux-rust.md` §2 — the Rust SDK layer a power user
  drops into when they outgrow the shipped surfaces.

--- END 23-user-ux-running-agents.md ---

# Batch REF23 — User UX: four surfaces + unified verb set across interfaces

**Refinement source**: `tmp/refinements/23-user-ux-running-agents.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- `docs/12-interfaces/` — four surfaces (CLI/TUI/Chat/Web), unified verb set, first-run flow, undo model.
- `docs/00-architecture/INDEX.md` — link user-UX chapter.
- `docs/17-lifecycle/` — session / resumption references.

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/23-user-ux-running-agents.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.


## Required vocabulary (verify)

The verify step greps for: `verb set|unified.*verb|four surfaces|CLI.*TUI.*Chat.*Web`

At least one changed file must mention it (case-insensitive) before the
batch is considered complete.

## Cross-references

Dependents — downstream refinements assuming this one has propagated
into docs: REF25, REF26, REF28, REF30

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
- Commit ready with message `refinements(REF23): User UX: four surfaces + unified verb set across interfaces`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
