# Refinements Batch REF25

Run id: run-20260416-221511
Attempt: 1
Model: gpt-5.4
Reasoning: high
Refinement source: /Users/will/dev/nunchi/roko/roko/tmp/refinements/25-domain-specific-agents.md
Target docs (candidates): docs/02-agents/ docs/12-interfaces/ docs/18-tools/

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

Suggested parallel split for batch `REF25`:

- worker: update `docs/02-agents/` with the six domain profiles framing.
- worker: add/update files under `docs/18-tools/` with per-domain tool sets.
- worker: update `docs/12-interfaces/` with profile-install workflow.

## Canonical refinement source

This is the verbatim refinement proposal that this batch must propagate
into `docs/`. Treat it as the authoritative source. Do not edit this
file; only edit docs under `docs/`.

--- BEGIN 25-domain-specific-agents.md ---

# Domain-Specific Agents

> **TL;DR**: Roko is a general-purpose agent toolkit, but most
> deployments will be domain-specific: coding agents, research
> agents, blockchain agents, data-engineering agents, ops agents.
> Each domain reuses ~80% of the kernel and customizes the
> remaining ~20% via roles, tools, gates, heuristics, and
> composer templates. This doc proposes six canonical domain
> profiles, shows what each needs from the kernel, and identifies
> two new subsystems (typed-context and chain-of-custody) that
> would unlock domains that are currently awkward.

> **For first-time readers**: A "profile" is a plugin bundle that wraps
> tier-1/2/3/4 extensions (see 17) for a specific domain. Users install
> a profile — coding, research, blockchain, data, ops, writing — and
> get a coherent starting point: tools, roles, gates, starter
> heuristics, composer templates. Two primitives this doc requires from
> the kernel are **TypedContext** (structured situation data) and
> **Custody** (chain-of-custody records for auditable actions). Read 17
> first for the plugin story; 14 for the heuristic story; 11 for HDC.

## 1. The domain matrix

| Domain | Core tools | Key gates | Heuristic sources | Memory shape |
|---|---|---|---|---|
| **Coding** | fs, cargo/npm/etc, git, mcp-code | unit, compile, clippy, diff | test outcomes, PR reviews | episodes + playbooks |
| **Research** | web, arxiv, pdf, note-taking | citation-check, factuality | paper claims, prior searches | dense heuristic library |
| **Blockchain** | rpc, signer, explorer, compiler | simulation, gas, invariant | historical exploits, docs | immutable audit trail |
| **Data/ML** | sql, pandas, jupyter, notebooks | schema, sample-check, metric | metric regressions | dataset fingerprints |
| **Ops/SRE** | kubectl, logs, metrics, runbook | dry-run, blast-radius | incident postmortems | runbook library |
| **Writing** | corpus, dictionary, style, citation | style, fact, tone | editorial feedback | voice fingerprint |

The kernel is the same. What differs:

- Which tools are present.
- Which gates are wired into the pipeline.
- Which heuristics are seeded.
- Which roles and templates are default.
- Which substrate features (e.g., chain-of-custody for blockchain)
  are enabled.

Plugin tiers from `17` already support this. Domain profiles are
plugin bundles — a curated set of Tier-1 (prompts), Tier-2
(profile), Tier-3 (tools), and Tier-4 (native) extensions that
ship together.

## 2. Coding agent (the default)

What Roko is best at today. Domain-specific reinforcements:

- **Tools**: file-system, version control, language-specific build
  systems (cargo/npm/pip/go). All exist.
- **Gates**: compile, unit, integration, clippy/lint, diff. All
  wired.
- **Heuristics**: from `14` §4 starter kit — flaky-test-logging,
  lockfile-on-merge-failure, etc. Starter library of ~30.
- **Roles**: researcher, planner, implementer, reviewer. Exist.

Gaps:

- **Code-graph awareness**: `roko-mcp-code` provides some; deeper
  integration with language servers would let the agent navigate
  semantically (rename-refactor safely, etc.).
- **Dependency-aware suggestions**: when the agent touches code,
  suggest updating callers; today it relies on the LLM noticing.

## 3. Research agent

Has partial support (`roko research *` subcommands). Expansions:

- **Tools**: arxiv API, Semantic Scholar, Papers With Code,
  Google Scholar, PDF extraction (use the pdf skill), citation
  manager integration (Zotero/BibTeX).
- **Gates**: citation-check (every claim cites a source),
  factuality (cross-check against fresh retrievals), novelty
  (not a duplicate of existing lit).
- **Heuristics**: seeded from `16-research-to-runtime.md` starter
  kit.
- **Memory**: Paper + Claim Engrams with Replication Ledger. Search
  is HDC-similarity rather than keyword.
- **Output modes**: literature review, annotated bibliography,
  research plan, replication report.

New subsystem need: **verified citations**. A citation is valid if
it points to a resolvable source and the quoted text actually
appears. A `CitationGate` that checks this should ship as part of
the research profile.

## 4. Blockchain agent

Least-supported today but high-leverage because mistakes are
catastrophic and audit trail is legally useful.

- **Tools**: Ethereum/Solana RPC, contract compiler (solc,
  anchor), block explorer, signer (with hardware-key support),
  simulator (tenderly, anvil), static analysis (slither, mythril).
- **Gates**:
  - **Simulation gate**: every proposed on-chain action is
    dry-run first; proceed only if the simulation succeeds *and*
    matches an explicit user-approved intent fingerprint.
  - **Gas gate**: proposed gas cost under a budget.
  - **Invariant gate**: contract invariants (maintained by a
    pluggable checker) still hold after the action.
  - **Blast-radius gate**: if the action touches funds above a
    threshold, require human approval.
- **Heuristics**: seeded from historical exploits (reentrancy
  patterns, integer overflows, missing access controls).
  Replication ledger against published audits.
- **Memory**: chain-of-custody — every transaction is an Engram
  with a witness (the actual on-chain receipt). Phase 2+
  `roko-chain` is purpose-built for this.

New subsystem need: **typed intents**. The user expresses intent
in typed form ("send N tokens from A to B with max gas G"); the
agent produces a transaction; the simulation verifies the
transaction matches the intent; then it's signed. Typed-intent
verification is a gate that blockchain domain fundamentally
requires.

## 5. Data / ML agent

- **Tools**: SQL (typed via sqlx-like introspection), pandas/polars,
  Jupyter kernel, notebook renderer, plotting, data
  profiling (great_expectations-style).
- **Gates**:
  - **Schema gate**: query doesn't violate the known schema.
  - **Sample-check gate**: materialize a small sample; look at
    distribution; reject if it's out of expected bounds.
  - **Metric-regression gate**: proposed change to a training
    pipeline must not regress key metrics beyond a threshold.
- **Heuristics**: from data-engineering best practices (nullable
  columns catch you, timezone drift is real, CSV encoding
  matters).
- **Memory**: dataset fingerprints (HDC encoding of schema +
  distribution summary), lineage between derived tables.

New need: **notebook-first workflow**. Roko should be able to
author, execute, and inspect Jupyter notebooks as first-class
artifacts. Notebooks can be Engrams; cells can be Pulses.

## 6. Ops / SRE agent

High-risk because mistakes affect running systems.

- **Tools**: kubectl (namespaced, dry-run default), logs (Loki,
  Elastic, CloudWatch), metrics (Prometheus), runbook retrieval,
  pager (PagerDuty, Opsgenie).
- **Gates**:
  - **Dry-run gate**: every action is dry-run; proceed only if
    diff is within expected scope.
  - **Blast-radius gate**: number of nodes/pods affected is under
    threshold without human approval.
  - **Change-window gate**: actions outside approved windows
    require override.
- **Heuristics**: from postmortems — the "we've been here before"
  database. Postmortems as Paper Engrams.
- **Memory**: incident archives, runbook executions, pattern
  library.
- **Modes**:
  - **Observer** (read-only, proposes fixes).
  - **Advisor** (proposes steps, waits for human).
  - **Executor** (acts, with guardrails).

New need: **explainable actions**. Every ops action should carry
a human-readable justification trace that's auditable after the
fact. Tie to `14-worldview-validation.md` (heuristic provenance)
and `16-research-to-runtime.md` (claim provenance).

## 7. Writing / content agent

- **Tools**: corpus search, style guide lookup, fact-check,
  citation manager, grammar/style (write-good, vale).
- **Gates**: style, fact, tone, plagiarism, length.
- **Heuristics**: from editorial feedback — "passive voice here,
  action here", "this paragraph introduces a new concept without
  defining it".
- **Memory**: *voice fingerprint* — an HDC encoding of the
  author's style, learned from their prior writing. Used to gate
  whether a generated draft "sounds like them."

New need: **stylistic fingerprinting**. An HDC encoder that takes
a text corpus and produces a fingerprint characterizing voice.
Drafts with fingerprint far from the author's get flagged.

## 8. Cross-domain shared subsystems

Looking at the domains side-by-side, two patterns recur:

### 8.1 Typed context

Every domain wants to express "the situation" in a structured way
so gates and heuristics can match on it. Today situations are
mostly free-text episode summaries. A `TypedContext` primitive:

```rust
pub struct TypedContext {
    pub domain: Domain,
    pub fields: BTreeMap<ContextKey, ContextValue>,
}

pub enum ContextValue {
    String(String),
    Int(i64),
    Float(f64),
    Hash(EngramHash),
    Fingerprint(HdcVector),
    List(Vec<ContextValue>),
    Nested(BTreeMap<ContextKey, ContextValue>),
}
```

Each domain profile declares its keys (e.g., coding declares
`language`, `repo_root`, `file_set`; blockchain declares `chain`,
`wallet`, `intent`). Gates and heuristics match on typed
predicates rather than string parsing.

This is the missing data primitive that holds domains together.

### 8.2 Chain of custody

Every domain has actions with consequences that should be auditable:
blockchain transactions, ops deploys, data pipeline changes,
published writing. A common `Custody` record:

```rust
pub struct Custody {
    pub action: ActionHash,
    pub who: PrincipalId,
    pub when: Timestamp,
    pub why: Vec<HeuristicId>,     // which heuristics influenced
    pub how: Vec<ClaimId>,         // which claims backed them
    pub approved_by: Option<PrincipalId>,
    pub simulation: Option<SimulationHash>,
    pub result: Option<ResultHash>,
    pub witness: Option<ChainWitness>,  // Phase 2+
}
```

Every domain benefits. Ops teams need it for compliance; blockchain
agents need it for dispute resolution; data teams need it for
lineage; writing needs it for editorial review. Shipping this
once, in `roko-core`, pays off in every domain.

## 9. Domain profiles as installable bundles

Following `17`, each domain is a *profile bundle*:

```
roko plugin install @roko/coding-profile
roko plugin install @roko/research-profile
roko plugin install @roko/blockchain-profile
# ...
```

A bundle is a tier-2 profile wrapping tier-1/3/4 extensions:

```toml
# @roko/coding-profile/profile.toml
name = "coding"
description = "Default coding agent profile"

tools = [
  "fs.read", "fs.write",
  "git.status", "git.diff", "git.commit",
  "cargo.build", "cargo.test", "cargo.clippy",
  "mcp-code.*",
]

roles = ["researcher", "planner", "implementer", "reviewer"]

gates = [
  { rung = "unit", id = "cargo.test" },
  { rung = "type", id = "cargo.check" },
  { rung = "style", id = "cargo.clippy" },
  { rung = "diff",  id = "roko.diff_gate" },
]

heuristics = "@roko/coding-heuristics-starter"
templates  = "@roko/coding-templates"
```

Users install a profile and get a coherent experience. Power users
customize by overriding specific tools/gates/heuristics while
keeping the rest of the profile intact. Profiles are themselves
versioned and can depend on minimum core versions.

## 10. Domain composition

The interesting case: **one project uses multiple domains**.
A blockchain startup's Roko instance might need both coding and
blockchain domains. Composition rules:

- **Tools merge**: union of tools from all installed profiles.
- **Roles merge**: union; if two profiles define the same role
  name, a collision warning fires.
- **Gates stack**: all gates from all profiles run on all tasks
  unless scoped. Scoping: `gates only in profile=<name>`.
- **Heuristics coexist**: all heuristics are available; routing
  picks based on HDC similarity of the situation.

This lets a team add a new domain without disrupting existing
workflow — it's additive until you explicitly wire it in.

## 11. The "agent for X" template

Community contribution pattern: someone builds an agent for their
domain (legal, medicine, accounting, infosec), packages it as a
profile bundle, publishes to the registry. Each published domain
benefits from the shared kernel AND from what the commons has
learned in adjacent domains.

This is where the Metcalfe's-law effect from `18` materializes:
every new domain profile expands what Roko can do, and every
domain profile benefits from the shared substrate.

## 12. What to ship first for domains

Priority:

1. **`TypedContext`** primitive in `roko-core`. Unblocks
   everything. One week.
2. **`Custody`** record + simple ops integration. Two weeks.
3. **Coding profile formalization**: most work already exists,
   package as a bundle. Three days.
4. **Research profile**: build on existing `roko research *`.
   Add citation-gate, paper-claim-heuristic integration. One week.
5. **Blockchain profile**: typed intents + simulation gate + chain
   witness scaffolding. Two weeks (partly Phase 2).
6. **Ops profile**: dry-run gate + blast-radius + postmortems as
   memory. Two weeks.
7. **Data / ML profile**: notebook support + schema gate. Two
   weeks.
8. **Writing profile**: voice fingerprint + style gates. One week.

Total about 2-3 months. At the end Roko has credible support for
six domains and a pattern for adding more.

## 13. Domain-specific evaluation suites

Each profile ships with a benchmark suite whose results go into the
replication ledger (16). Examples:

- **Coding**: a set of known bugs in OSS repos with frozen SHAs.
  Agent scores = time-to-green + token count + correctness.
- **Research**: a curated set of arxiv abstracts with known follow-up
  papers. Agent proposes follow-ups; match rate is the score.
- **Blockchain**: a set of known vulnerable contracts. Agent detection
  rate and false-positive rate are the scores.
- **Data**: a set of dirty datasets with known issues (type
  mismatches, duplicates, outliers). Agent's diagnosis report is
  compared to ground truth.
- **Ops**: a set of simulated incidents with known root causes. Time
  to correct diagnosis is the score.
- **Writing**: a set of style-fingerprinted corpora. Draft fidelity
  to target voice is the score (measured by HDC similarity to
  author's fingerprint, per §7).

These suites *also* serve as the load test for superlinear scaling
(15 §12): run them periodically, measure slope of score vs deployment
age. A flat slope signals a broken feedback loop.

## 14. Concrete starter heuristics per domain

Beyond the structural items in §2–§7, each profile ships a small
starter heuristic library. Illustrative examples (format follows 14 §2):

**Coding**:
- `h.code.001` — "When unit tests pass locally but fail in CI, check
  for dependency-version drift first." Precondition: `GateRecentlyFailed(unit, intermittent=true)`.
- `h.code.002` — "When a compile error mentions a trait bound, the
  next action is usually adding `impl` or adjusting generics, not
  modifying the trait itself."
- `h.code.003` — "Before bumping a dep in Cargo.toml, run `cargo tree -d`
  to check what else depends on it."

**Research**:
- `h.research.001` — "Claim whose effect size is reported without CI
  in the abstract is a replication risk; flag for falsifier
  sharpening."
- `h.research.002` — "If a paper cites its own preprint as the
  replication, treat it as untested."

**Blockchain**:
- `h.chain.001` — "A contract that modifies state before external
  call — probably reentrant. Run slither before signing."
- `h.chain.002` — "Gas estimation from mainnet node differs from
  fork; reject proposal if gap > 15%."

**Ops**:
- `h.ops.001` — "When error rate spikes but latency is flat,
  upstream is likely the cause. Check there before touching
  anything."
- `h.ops.002` — "Rolling restart before the change window ends:
  don't."

**Writing**:
- `h.write.001` — "When a draft's HDC fingerprint distance to the
  author's voice exceeds 0.35, the tone probably slipped."

**Data**:
- `h.data.001` — "If a column's null rate doubles overnight,
  upstream schema change is most likely. Check the source system's
  changelog before touching the loader."

These aren't exhaustive — they're seeds. Each profile ships with
~20-30 heuristics. Users' own deployments calibrate them and grow
new ones organically via `/learn` from episodes (28 §4).

## 15. Profile composition at runtime

Installing two profiles simultaneously (coding + blockchain, say)
needs explicit conflict resolution. Proposed rules (formalizing
§10):

```toml
# When merging profiles, conflicts resolve via profile priority.
[profile_resolution]
# Lower priority loses on key conflicts.
order = ["coding", "blockchain", "research"]

# Gates cumulate unless explicitly scoped.
gate_mode = "cumulate"    # or "scope_by_task_tag"

# Tools merge; duplicates (same id) use the first declaring profile.
tool_mode = "first_wins"

# Heuristics coexist; routing picks by HDC fit.
heuristic_mode = "coexist"

# Role prompts: collision warning if same name declared twice.
role_mode = "warn_on_collision"
```

A `roko profile check` command validates composition before use,
reports conflicts, and offers resolutions. CI-friendly exit codes
let CD pipelines catch bad profile combinations early.

## 16. Second TypedContext example — blockchain

To show the TypedContext primitive from §8.1 across domains:

```rust
// Coding TypedContext
TypedContext {
    domain: Domain::Coding,
    fields: [
        (key!("language"),   ContextValue::String("Rust".into())),
        (key!("repo_root"),  ContextValue::String("/workspace".into())),
        (key!("file_set"),   ContextValue::List(vec![
            ContextValue::String("src/lib.rs".into()),
            ContextValue::String("src/core.rs".into()),
        ])),
        (key!("last_gate"),  ContextValue::String("compile:fail".into())),
    ].into_iter().collect(),
}

// Blockchain TypedContext
TypedContext {
    domain: Domain::Blockchain,
    fields: [
        (key!("chain"),      ContextValue::String("ethereum-mainnet".into())),
        (key!("wallet"),     ContextValue::String("0xABC...".into())),
        (key!("intent"),     ContextValue::Nested([
            (key!("action"), ContextValue::String("transfer".into())),
            (key!("to"),     ContextValue::String("0xDEF...".into())),
            (key!("amount"), ContextValue::String("1.5 ETH".into())),
            (key!("max_gas"), ContextValue::Int(200_000)),
        ].into_iter().collect())),
        (key!("simulation"), ContextValue::Hash(sim_hash)),
    ].into_iter().collect(),
}
```

The same data shape serves both domains. Gates and heuristics match
against typed keys; no domain has to parse free-text situation
descriptions. Third-party domains register their own key schemas
and share the same match infrastructure.

## 17. Cross-references

- Plugin SPI that domains ride on: `17-plugin-extension-architecture.md`.
- Starter heuristic templates: `14-worldview-validation.md` §4.
- Replication of domain claims: `16-research-to-runtime.md` §8.
- Custody / safety spine: `32-safety-sandbox-provenance.md` §5.
- Deployment considerations for domain-specific setups:
  `24-deployment-ux.md` §2.
- Observability per domain:
  `33-observability-telemetry.md` §5.

--- END 25-domain-specific-agents.md ---

# Batch REF25 — Domain-specific agents: six profiles across agents chapter

**Refinement source**: `tmp/refinements/25-domain-specific-agents.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- `docs/02-agents/` — six domain profiles (coding, research, blockchain, data, ops, writing).
- `docs/12-interfaces/` — profile-install workflow (tier-2 plugins).
- `docs/18-tools/` — per-domain tool sets.

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/25-domain-specific-agents.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.


## Required vocabulary (verify)

The verify step greps for: `domain profile|TypedContext|Custody`

At least one changed file must mention it (case-insensitive) before the
batch is considered complete.

## Cross-references

Dependents — downstream refinements assuming this one has propagated
into docs: REF32

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
- Commit ready with message `refinements(REF25): Domain-specific agents: six profiles across agents chapter`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
