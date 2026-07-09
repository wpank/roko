# Refinements Batch REF24

Run id: run-20260416-221511
Attempt: 1
Model: gpt-5.4
Reasoning: high
Refinement source: /Users/will/dev/nunchi/roko/roko/tmp/refinements/24-deployment-ux.md
Target docs (candidates): docs/19-deployment/ docs/12-interfaces/

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

Suggested parallel split for batch `REF24`:

- worker: add/update files under `docs/19-deployment/` covering the five
  deployment shapes, profiles, secrets, state portability, multi-tenancy.

## Canonical refinement source

This is the verbatim refinement proposal that this batch must propagate
into `docs/`. Treat it as the authoritative source. Do not edit this
file; only edit docs under `docs/`.

--- BEGIN 24-deployment-ux.md ---

# Deployment UX

> **TL;DR**: Roko should be deployable in five distinct shapes —
> laptop-local, single-server, container, clustered, and edge —
> with almost no code differences between them. The single Rust
> binary is the first shape; everything else is a configuration
> layer on top of it. This doc proposes deployment profiles,
> secret handling, state portability, and observability so
> moving between shapes is a one-day task, not a rewrite.

### For first-time readers

Recurring terms:

- **Substrate** — durable store (Engrams): SQLite on laptop,
  Postgres/object store at scale. Trait-level; swap with config.
- **Bus** — ephemeral stream (Pulses): in-memory by default,
  NATS/Redis/Kafka for clustered. Trait-level; swap with config.
- **Profile** — a bundle of defaults for a deployment shape
  (laptop / single-server / container / clustered / edge). One
  binary, five profiles.
- **State portability** — substrate + bus queues + config
  exported/imported as one signed archive. See §4.

## 1. The shapes

### 1.1 Laptop-local (developer mode)

- Single user, single machine, `./.roko` in the project directory.
- Models: local (Ollama/LM Studio) and/or cloud APIs with keys.
- No HTTP exposure unless explicitly `roko serve`.
- All plugins are local files.

This is the default and the most common. Works today. Improvements
focused on `23` (user UX) and the `roko init` flow.

### 1.2 Single-server (team shared)

- A small team shares one Roko instance on a box they control.
- `roko serve` exposed to LAN or VPN.
- State in a shared directory or a lightweight DB (SQLite or
  PostgreSQL).
- Plugin set curated for the team.

Key addition: **multi-user auth**. A minimal identity layer that
tags episodes, heuristics, and decisions with who-initiated-what.

### 1.3 Container (cloud-host)

- Docker image, runs on anyone's container runtime.
- Stateful volume mount.
- Environment-based configuration.
- Probes (liveness, readiness) wired to the control plane.

### 1.4 Clustered (scale-out)

- Multiple Roko instances behind a load balancer.
- Shared Substrate (Postgres or object-store backed).
- Shared Bus (NATS, Redis Pub/Sub, or Kafka).
- Sticky routing for long-lived sessions; stateless otherwise.

### 1.5 Edge (embedded, serverless, WASM)

- Minimal feature set.
- No persistence or read-only memory.
- Called per-request, returns, disappears.
- Targets: Cloudflare Workers, Deno Deploy, Lambda, WASM
  runtimes.

Each shape can be reached by the same binary with a different
config. The deployment experience is picking a shape, not
rebuilding.

## 2. Deployment profiles

A new config concept: `profile`.

```toml
# roko.toml
profile = "single-server"   # one of: laptop, single-server, container, clustered, edge

[profile.single-server]
listen      = "0.0.0.0:6677"
auth        = "basic"
substrate   = { kind = "sqlite", path = "/var/lib/roko/state.db" }
bus         = { kind = "in-memory" }
```

Profiles bundle defaults. A user pins a profile and overrides only
what they care about. Profiles are user-writable and shippable as
tier-2 plugins (`17` §2).

## 3. Secrets story

Today model API keys live in env vars or config files. For broader
deployment, we need:

- **Layered resolution**: env → config → OS keychain → secret store
  (Vault / AWS Secrets Manager / 1Password CLI).
- **Never-logged**: secrets tagged in config schemas so log
  sanitization is automatic.
- **Rotation-friendly**: the Roko process can pick up a new secret
  without restart (subscribe to secret-change events).
- **Per-role secrets**: the researcher might have a Perplexity
  key; the implementer doesn't. Role-scoped injection.

Proposed CLI:

```bash
roko secret set anthropic.api_key
roko secret get anthropic.api_key     # requires confirmation
roko secret list
roko secret rotate anthropic.api_key
```

Behind the scenes, backed by the OS keychain for laptop mode, by
`vault` or equivalent for server mode. Swappable via a `SecretStore`
trait.

## 4. State portability

The single most important cross-shape concern. Users need to:

- Move state from laptop to server without surgery.
- Back up state.
- Reset state (for a clean experiment).
- Split state (project A vs project B).

Proposed:

```bash
roko state export <file.tar.zst>   # serializes substrate, bus queues, config
roko state import <file.tar.zst>
roko state split --by project      # produces N archives
roko state merge <file1> <file2>   # merges archives with conflict policy
roko state gc --dry-run            # shows what demurrage would drop
```

State archives should be content-addressed, versioned, and signed
(for integrity). Users can audit what they're about to import. A
commons member can share "their heuristic state" as a signed
archive.

## 5. Observability is part of deployment

Every Roko shape needs:

- **Structured logs** (JSON) on stderr by default.
- **Prometheus-compatible metrics** on `/metrics`.
- **OpenTelemetry traces** with spans around each operator.
- **Health probes**: `/healthz` (liveness), `/readyz` (readiness).

And *domain-specific* metrics that aren't in existing dashboards:

- `roko.c_factor` (gauge)
- `roko.demurrage.balance_p50` / `_p95` (histogram)
- `roko.heuristic.calibration_brier_score` (gauge)
- `roko.gate.pass_rate` by rung (counter rate)
- `roko.bus.pulses_per_second` by topic (counter rate)
- `roko.substrate.query_latency_p99` by kind (histogram)

These are uniquely Roko's; every deployment exposes them; they
can be scraped by existing Prometheus/Grafana setups without
special integration.

## 6. Zero-downtime upgrades

For single-server and clustered shapes, the goal is rolling
upgrades without losing work.

- **Graceful shutdown**: SIGTERM causes Roko to stop accepting new
  tasks, finish running ones, checkpoint state, exit.
- **Warm restart**: a new process reads the checkpoint, resumes.
- **Cluster rolling**: load balancer drains one node, the replaced
  node rejoins, repeat.

Build on existing `--resume` support in the plan executor; extend
to cover agents mid-turn.

## 6.5 Metrics and log aggregation

`/metrics`, `/healthz`, and structured stderr are the low layer.
Most operators want them flowing into an existing stack. Supported
paths, none of which fork the binary:

- **Prometheus + Grafana**: the default. Ship a Grafana dashboard
  JSON under `packaging/dashboards/roko.json` covering c-factor,
  gate pass-rates, demurrage balance, pulse throughput.
- **OpenTelemetry**: `RUST_LOG` plus
  `OTEL_EXPORTER_OTLP_ENDPOINT=https://otel.example.com` turns on
  OTLP export for traces, metrics, and logs. No additional config
  needed — the operator spans from §5 are already OTel-native.
- **ELK / OpenSearch**: switch stderr to `--format=json` (ECS
  field names) and point Filebeat at it. Index mapping shipped
  under `packaging/elk/roko-ecs-template.json`.
- **Loki / Grafana Agent**: the same JSON works as-is; promtail
  labels picked up via `--log-labels tenant=${ROKO_TENANT}`.
- **Datadog / New Relic / Honeycomb**: OTLP path covers all three.
- **Audit sink**: every Custody record (`25-domain-specific-agents.md`
  §8.2) is emitted as a separate structured log line with a
  stable schema, so SIEM ingestion doesn't need to know anything
  about Roko internals.

Rules:

- Metric names are `roko.<subsystem>.<thing>`; no shape-specific
  names (no `roko.clustered.*` vs `roko.laptop.*`). The shape is
  a label, not a prefix.
- Labels are low-cardinality by default. Tenant and role go on
  metrics; episode-hash stays in traces only.
- Sampling: high-volume subsystems (Bus throughput, token streams)
  ship with a tail-based sampler on by default; per-subsystem
  overrides via `[observability.sampling]` in `roko.toml`.

See also `27-realtime-event-surface.md` §4 — the same cursor
format is reused for log-tail subscriptions, so `roko logs follow
--tenant acme` and a Grafana Loki query see identical events.

## 7. Platform-specific shapes

### 7.1 Kubernetes

Ship a Helm chart. Support StatefulSet for single-server, Deployment
+ HPA for clustered. Config via ConfigMap; secrets via K8s Secret;
volumes for state.

Opinionated values file targeting the common case:

```yaml
roko:
  replicas: 3
  profile: clustered
  persistence:
    size: 20Gi
    storageClass: fast-ssd
  secrets:
    anthropic:
      existingSecretKey: roko-anthropic-key
  ingress:
    enabled: true
    host: roko.internal.example.com
```

### 7.2 Docker Compose

For single-server teams without K8s. A ready-made compose file with
Roko + Postgres + NATS. Two commands to be running.

A multi-stage `Dockerfile` keeps the production image small,
reproducible, and free of the toolchain. Sketch:

```dockerfile
# --- build stage ---
FROM rust:1.91-slim AS build
WORKDIR /src
RUN apt-get update && apt-get install -y --no-install-recommends \
      pkg-config libssl-dev ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Cache deps: copy manifests first, build a skeleton.
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN cargo fetch --locked

# Real build against a warm dep cache.
ARG ROKO_PROFILE=release
RUN cargo build --locked --profile ${ROKO_PROFILE} --bin roko && \
    strip target/${ROKO_PROFILE}/roko

# --- runtime stage ---
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
WORKDIR /app
COPY --from=build /src/target/release/roko /usr/local/bin/roko
COPY packaging/docker/roko.toml /etc/roko/roko.toml

ENV ROKO_CONFIG=/etc/roko/roko.toml \
    ROKO_STATE_DIR=/var/lib/roko \
    ROKO_LOG_FORMAT=json \
    RUST_LOG=info

VOLUME ["/var/lib/roko"]
EXPOSE 6677
USER nonroot:nonroot

HEALTHCHECK --interval=15s --timeout=3s --start-period=10s \
  CMD ["/usr/local/bin/roko", "probe", "readyz"]

ENTRYPOINT ["/usr/local/bin/roko"]
CMD ["serve", "--profile", "container"]
```

Notes:

- Builder is `rust:1.91-slim` to match the MSRV flagged in
  CLAUDE.md. CI pins the digest.
- Runtime is `distroless/cc` — no shell, no package manager, no
  setuid. ~40 MB image; drops to ~15 MB with musl static binary
  (`packaging/docker/Dockerfile.musl`).
- `/var/lib/roko` is the state volume; maps to the archive used
  by `roko state export` (§4). Backup/restore is cp-in, cp-out.
- `HEALTHCHECK` uses the same `/readyz` probe K8s does (§7.1), so
  Compose and Helm agree on health semantics.
- Secrets never bake into the image; they arrive via env (§3) or
  a mounted tmpfs.

### 7.3 systemd unit

For single-server teams on bare metal. A canonical unit file in
`packaging/systemd/` with restart policy, journald logging,
capability restrictions.

### 7.4 macOS launchd

For Roko running as a background service on a Mac.

### 7.5 WASM targets

For edge deployment, a `roko-wasm` binary compiled to wasm32-wasi
with the smallest possible feature set (no filesystem, no local
models, no plugin registry — just core + HTTP).

Each of these is a packaging artifact, not a code fork. Same Rust,
different front door.

## 8. Multi-tenancy

In single-server and clustered deployments, multiple users share
one Roko. Isolation requirements:

- **Memory isolation**: each tenant has their own Substrate scope;
  heuristics don't cross-contaminate unless explicitly shared.
- **Auth**: OIDC (Google Workspace, Microsoft, Okta) via a
  pluggable `Auth` trait. Plus API keys for machine users.
- **Quotas**: per-tenant token/dollar/episode budgets.
- **Role limits**: some roles or tools off by default for
  untrusted tenants.

Build on the existing role-auth in `roko-agent/src/safety/`. The
safety layer generalizes nicely from role → tenant × role.

### 8.1 Auth-header → tenant mapping

A concrete wiring that turns an inbound HTTP request into a
`TenantCtx` the kernel uses for substrate scoping and budget
enforcement. One `roko.toml` stanza, zero custom code:

```toml
[auth]
mode = "oidc"

[auth.oidc]
issuer        = "https://auth.example.com/"
audience      = "roko"
jwks_uri      = "https://auth.example.com/.well-known/jwks.json"
cache_ttl     = "10m"

# Claim-to-tenant projection. First matching rule wins.
[[auth.tenant_rules]]
# Canonical: an explicit tenant claim issued by the IdP.
claim   = "https://roko.dev/tenant"
tenant  = "${value}"
roles   = "${claims.roles[*]}"

[[auth.tenant_rules]]
# Google Workspace: hd claim is the hosted domain.
claim   = "hd"
tenant  = "workspace:${value}"
roles   = ["viewer"]       # default; overridden by group rules below

[[auth.tenant_rules]]
# Fallback: derive from email domain.
claim   = "email"
regex   = "^[^@]+@(?P<domain>.+)$"
tenant  = "email:${domain}"
roles   = ["viewer"]

[[auth.group_rules]]
# Map IdP groups to Roko roles.
group_claim = "groups"
mapping = {
  "roko-admins"      = "admin",
  "security-reviewers" = "reviewer",
  "engineering"      = "implementer",
}
```

Request flow:

1. Ingress validates `Authorization: Bearer <jwt>` against the
   JWKS (cached for `cache_ttl`).
2. The first `tenant_rule` that matches emits `TenantId`, e.g.
   `workspace:acme.com`. `group_rules` layer roles on top.
3. The `TenantCtx` is attached to the request and propagated as a
   span attribute (see §6.5) and as a prefix on every Substrate
   key: `tenant:workspace:acme.com/engram/<hash>`.
4. Quotas (§8) lookup uses `TenantId` as the aggregation key.

Machine users bypass OIDC via `Authorization: Bearer roko_pat_...`
(personal access tokens). PATs carry tenant and role at creation
time (`roko token create --tenant acme --role implementer`) and
live in the OS keychain / secret store (§3).

A header-only shortcut is supported for air-gapped deployments
behind a trusted reverse proxy:

```toml
[auth.headers]
tenant_header = "X-Roko-Tenant"
role_header   = "X-Roko-Role"
trust_chain   = ["10.0.0.0/8"]   # only accept from this CIDR
```

This path is off by default; enabling it explicitly is required
because it shifts trust to the upstream proxy.

## 9. The migration path for existing state

Users with `.roko/` directories from v1 need to keep their
episodes, plans, heuristics when upgrading to v2 (the
post-refinements kernel). Migration rules:

- **Automatic**: a version stamp in `.roko/meta` triggers
  migration on first run.
- **Non-destructive**: the old state is preserved as
  `.roko/.pre-v2/`.
- **Auditable**: `roko migrate --dry-run` shows what changes.
- **Reversible**: `roko migrate --rollback` if something goes
  wrong.

This is a 3-day project of careful work. It matters because
without it, every user has to choose between "stay on v1" and
"lose my accumulated state." Either is a bad choice for them.

## 10. Cost visibility

A Roko deployment is going to spend money on LLM APIs. Users need
clarity:

- **Live cost counter**: the TUI and Web UI show `$X.YZ spent
  this session`.
- **Budget caps**: `--budget 5` limits a run to $5 of API spend;
  the cascade router prefers cheaper models as budget depletes.
- **Per-task breakdown**: after a run, a table of cost per task,
  per agent, per model.
- **Historical**: `roko cost report --period 7d` for team-level
  understanding.

This is both a UX and a trust property. Users who can't see what
they're spending will either stop using the tool or feel
uncomfortable using it.

## 11. Air-gapped and on-prem realities

Some deployments cannot reach the public internet. For them:

- **Local models supported end-to-end** (already in `roko-agent`).
- **Plugin registry mirror**: an on-prem registry serving
  verified plugins.
- **Heuristic commons mirror**: imports from a curated,
  organization-internal commons rather than public.
- **Telemetry off by default**: no phoning home.

None of this requires a forked binary. Feature flags in the
profile.

## 12. The deployment UX pitch

"Single Rust binary, five shapes. Configuration is declarative.
State is portable. Secrets are layered. Observability is
standard-compatible. Upgrades are rolling. Multi-tenancy is
explicit. Air-gap works."

This is the DevOps side of the "world-class" claim. Deployment
can feel like a known-quantity infrastructure project (Postgres,
Redis, Nginx) rather than a novel-framework adoption risk. That
feeling is a moat of its own — it's what gets Roko approved by
platform teams rather than blocked by them.

## 13. What to ship first

Priority:

1. **Single-binary with profile config**. Already 80% there;
   needs the `profile` concept and a cleanup pass.
2. **`roko state export/import`**. Unlocks laptop-to-server
   migration. One week.
3. **Basic Docker image** (scratch + static musl binary).
   Two days.
4. **Docker Compose bundle** with Postgres + NATS. Three days.
5. **Helm chart**. One to two weeks.
6. **Secrets CLI and keychain integration**. One week.
7. **Cost visibility**. One week.

After these, Roko is deployable in all five shapes. Further
polish (WASM, air-gap, multi-tenancy maturation) is iterative.

--- END 24-deployment-ux.md ---

# Batch REF24 — Deployment UX: five shapes across deployment chapter

**Refinement source**: `tmp/refinements/24-deployment-ux.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- `docs/19-deployment/` — five deployment shapes, profiles, secrets, state portability, multi-tenancy.
- `docs/12-interfaces/` — references to deployment-specific configuration.

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/24-deployment-ux.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.


## Required vocabulary (verify)

The verify step greps for: `laptop|single.?server|container|clustered|edge`

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
- Commit ready with message `refinements(REF24): Deployment UX: five shapes across deployment chapter`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
