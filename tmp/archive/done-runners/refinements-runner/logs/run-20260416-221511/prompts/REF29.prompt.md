# Refinements Batch REF29

Run id: run-20260416-221511
Attempt: 1
Model: gpt-5.4
Reasoning: high
Refinement source: /Users/will/dev/nunchi/roko/roko/tmp/refinements/29-web-ui-architecture.md
Target docs (candidates): docs/12-interfaces/

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

Suggested parallel split for batch `REF29`:

- worker: add/update files under `docs/12-interfaces/` for the five-page
  first-party web UI (Home, Chat, Plans, Beliefs, Settings).

## Canonical refinement source

This is the verbatim refinement proposal that this batch must propagate
into `docs/`. Treat it as the authoritative source. Do not edit this
file; only edit docs under `docs/`.

--- BEGIN 29-web-ui-architecture.md ---

# Web UI Architecture

> **TL;DR**: A web UI should sit on top of StateHub (`26`) and the
> realtime surface (`27`) as a client, not as a server in its own
> right. This doc proposes a minimal first-party UI (3 to 5 core
> pages) built with a deliberate tech stack (SvelteKit or React
> via Vite, Tailwind, reactive store synced to StateHub),
> shippable in the same release as the backend, and extensible
> via the same plugin mechanism. Not a complete SaaS — a
> reference implementation that other teams can fork or extend.

> **For first-time readers**: This doc proposes a first-party web UI
> for Roko — five pages (Home, Chat, Plans, Beliefs, Settings), built
> on StateHub projections over the realtime event surface. The
> emphasis is "deliberate small" rather than "kitchen sink." A
> reference implementation that other teams can fork. Read 26 and 27
> first — the web UI is a consumer of the projection layer and
> realtime wire protocol defined there.

## 1. What the web UI is *for*

Not everyone opens a terminal. Not every operator installs a Rust
binary. The web UI serves:

1. **First-time curious users** who want to see Roko in action
   without installing anything.
2. **Non-developer stakeholders** (PMs, managers, executives) who
   want to observe without operating.
3. **Multi-user teams** where a shared single instance needs a
   shared visualization.
4. **Presentations and demos** where a browser window beats a
   terminal.
5. **Mobile viewing** — someone on a phone wants to see if the
   agent is done with the task they started this morning.

Non-goals: replicating the TUI's every feature, being a full IDE,
replacing the CLI for power users.

## 2. Tech stack

Pick a stack with a long-term UX story:

### 2.1 Framework

**SvelteKit** or **React + Vite**. Either works. Opinions:

- SvelteKit: smaller bundles, simpler reactivity, better for a
  dashboard-heavy UI.
- React + Vite: bigger ecosystem, more hires know it, better for
  an expanding UI.

Recommendation: SvelteKit for the reference implementation. Easy
to port later if a bigger team prefers React.

### 2.2 Styling

**Tailwind CSS**. Ubiquitous, consistent, enables fast iteration.
Supplement with **shadcn-svelte** (or shadcn/ui if React) for
high-quality primitives.

### 2.3 State

One **reactive store per StateHub projection**. The client library
from `27` wraps the realtime subscription; each store reflects
the current projection state and updates on deltas.

```ts
import { writable } from 'svelte/store';
import { roko } from '$lib/roko';

export const cohortHealth = writable<CohortHealthState | null>(null);
roko.subscribe("projection:cohort_health", {}, (msg) => {
  if (msg.type === "state") cohortHealth.set(msg.payload);
  if (msg.type === "delta") cohortHealth.update(s => applyDelta(s, msg.payload));
});
```

Same pattern for every projection. Pages just subscribe to stores
and render.

### 2.4 Charts

**visx**, **Observable Plot**, or **uPlot**. Declarative, fast,
good mobile support. Plot c-factor time series, balance
histograms, gate pass rates.

### 2.5 Editors

**CodeMirror 6** for inline code editing (diffs, patches). **Tiptap**
or **Lexical** for rich text (PRDs, heuristic descriptions).

## 3. The core pages

Ship exactly these. Resist scope creep.

### 3.1 Home / Pulse

Landing page. Five tiles:

- **System pulse**: current cohort c-factor (big gauge).
- **Active tasks**: tasks currently running with live progress.
- **Recent episodes**: last ~10 with one-line summaries.
- **Cost meter**: session-to-date spend.
- **Alerts**: gate failures, circuit breakers, demurrage extremes.

All tiles are live-updating. This is the "is everything OK?"
glance. A mobile-friendly view of this tile collection is the
default.

### 3.2 Chat

The interactive surface. Full-duplex via WebSocket. Features:

- Streaming token rendering.
- Inline diffs for proposed changes with apply/copy/edit.
- Slash commands (same set as CLI, `28` §4).
- File drop for context.
- Agent switch in the header (`@researcher`, `@implementer`).
- Voice input button for accessibility.
- Markdown + syntax highlighting.
- Replay link on every episode cite.

This is the most important page. 60% of usage will be here.

### 3.3 Plans

Tree/DAG visualization of plans and tasks.

- **Plan list** on the left; selected plan's DAG on the right.
- Nodes colored by status (pending, running, passed, failed).
- Clicking a task opens its episode trail.
- Drag-to-reorder tasks that haven't started.
- "Execute" button on non-running plans.
- Breakpoints: mark a task as "pause here, require approval."

This page is where Roko's plan-driven differentiation shows up
visually. It should be *beautiful*.

### 3.4 Beliefs

The heuristic + worldview browser.

- **Heuristics table** with calibration CIs, last trial,
  provenance.
- **Worldviews** as clustered views with dominant heuristics.
- **Replication ledger** with paper claims and our-vs-their
  effects.
- Challenge / retire / edit buttons.

This page communicates Roko's distinctive commitment to
empirical, inspectable belief. It's the *aha* page for skeptics.

### 3.5 Settings

Minimal. Covers:

- Model & API key management (delegates to `roko secret` via API).
- Profile selection (coding / research / blockchain / etc.).
- Gate thresholds (with reset-to-adaptive button).
- Plugin management (list, enable/disable, install from registry).
- Cost budgets.

Nothing exotic. Each settings change is an API call to the
control plane; the StateHub picks up the change and everyone
sees it immediately.

## 4. Component library

A reusable set of components lives in `@roko/ui` (or similar):

- `<CFactorGauge>` — the signature widget.
- `<EpisodeCard>` — stylized episode summary with citation trail.
- `<TaskNode>` — plan-DAG node with status and controls.
- `<HeuristicRow>` — calibration histogram + provenance.
- `<GateBadge>` — colored rung indicator.
- `<CostMeter>` — budget vs spend with warning states.
- `<ReplayTrack>` — timeline scrubber for an episode.
- `<DiffView>` — per-hunk diff with accept/reject.
- `<AgentAvatar>` — shows role, status, current action.

These are the building blocks. Third-party dashboards can import
them via `@roko/ui` and build custom pages.

## 5. Theming

- **Light + dark** baseline.
- **High-contrast** mode for accessibility.
- **Printable** CSS for exporting a replication report, PRD, or
  incident summary.
- Users can override via CSS variables. No proprietary theming
  system.

## 6. Routing and URLs

Every page is deep-linkable. Hash fragments for in-page state:

- `/plans/<slug>` — plan view
- `/plans/<slug>/task/<id>` — task focus
- `/beliefs` — heuristic list
- `/beliefs/h/<id>` — heuristic detail
- `/chat/<session>` — chat at a specific session
- `/episodes/<hash>` — episode detail

Users can share URLs. Replay links work. Screenshots come with
context.

## 7. Authentication

Three modes:

- **No-auth** (`--allow-any`): local dev only.
- **Basic auth**: for small teams. HTTP Basic over HTTPS.
- **OIDC**: for real deployments. Google Workspace, Microsoft,
  Okta, Authentik.

Session stored in an HTTP-only cookie; CSRF protection on all
mutations. The frontend never sees an API key.

## 8. Offline and progressive

The shell app is a PWA. Service worker caches assets. On
reconnect, StateHub catches up with cursors. Short outages don't
break the experience.

## 9. Accessibility

- Semantic HTML first, ARIA second.
- Every button and link reachable by keyboard.
- Screen-reader tested with a checklist (NVDA, VoiceOver).
- Focus outlines visible.
- Reduced motion respected.
- Internationalization: English default; every user-facing string
  in a resource bundle so translation is possible.

## 10. Mobile viewing (not full mobile app)

- Responsive layouts on all pages.
- Home / Pulse is the primary mobile view.
- Chat works on mobile but is read-biased (typing long prompts on
  a phone is not the goal).
- Plans DAG scrollable on mobile; horizontal focus is fine.

## 11. Extension points

### 11.1 Custom tiles on Home

Plugins can contribute Home-page tiles by registering a component
and a projection:

```ts
registerTile({
  name: "my-custom-tile",
  projection: "projection:my_metric",
  component: MyTile,
  size: "medium",
});
```

The registry fetches the component bundle lazily. Tiles are
sandboxed (iframe or shadow DOM) to prevent CSS bleed.

### 11.2 Custom pages

A plugin can register a top-level route. Users opt-in from
Settings. Third-party pages share the component library but can't
reach cross-origin resources.

### 11.3 Custom visualizations in existing pages

A plugin can register an alternative visualization for a projection
(e.g., a Sankey for lineage). Users choose from a dropdown.

## 12. Performance

Targets:

- **Time to interactive** on Home: under 2 seconds on a laptop.
- **Bundle size**: under 300 KB for the shell; pages lazy-load.
- **Memory**: under 150 MB browser memory even after an hour of
  chat.
- **Render budget**: no single update should block the main
  thread more than 16 ms (60 fps).

Tight budgets. Achievable with SvelteKit + careful component
structure. React is harder but doable.

## 13. Development experience

- Run `npm run dev` next to `cargo run -- serve` for full live
  development.
- Storybook (or equivalent) for every component — inspect in
  isolation.
- Typed schema from StateHub (codegen) so client types match
  server.
- Playwright for end-to-end tests.
- Vitest for unit tests.

## 14. What a non-developer sees on day one

```
[Home]
  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
  │   c-Factor   │  │ Active Tasks│  │    Cost     │
  │    0.76 ↑    │  │      3      │  │   $0.42     │
  │ (24h trend)  │  │ 2 passing   │  │   / $5.00   │
  └──────────────┘  └──────────────┘  └──────────────┘

  [Recent episodes]
    14:22  fix failing test in tests/core.rs       ✓  1.2m
    13:50  implement rate limiter                   ✓  4.8m
    13:12  investigate flaky integration test      ?  in progress

  [Alerts]
    ⚠ gate 'clippy' pass rate dropped 14% in last hour
```

A person with no context understands that the agent is
working, how expensive it's being, and whether anything needs
attention. That understanding is the product win. Most agent
frameworks don't try to produce it; Roko's substrate makes it
trivial once the UI exists.

## 15. Why not "just use the CLI"

The CLI is great. But:

- Non-developers can't or won't use CLIs.
- Mobile users can't use CLIs.
- Demos and board presentations don't work in CLIs.
- Observation and operation are different workflows; a web UI
  optimizes for observation.

The web UI earns its keep in those cases even if developers live
in the CLI.

## 16. Shipping sequence

1. **SvelteKit shell + auth + realtime connection**. Two weeks.
2. **Home + Chat pages**. Two weeks.
3. **Plans page**. Two weeks.
4. **Beliefs page**. Two weeks.
5. **Settings page**. One week.
6. **Component library polish + docs**. Two weeks.
7. **Accessibility audit**. One week.

A quarter of focused work for a credible reference web UI.
Shippable milestones at each step — Home + Chat alone would be
a release.

## 17. The big idea

Don't overbuild. Five pages, done well, built on a clean
StateHub + realtime surface, exposing a component library for
teams that want to fork. That's a product *and* a platform move:
the reference UI is both directly useful and a demonstration of
what the external API can do.

## 18. Page-level state ownership

Clear ownership rules so pages don't race each other:

- Each page owns its **local view state** (scroll position, expanded
  rows, input buffer). Not persisted across sessions.
- StateHub projections own **shared application state**. Pages
  subscribe, never mutate directly.
- Mutations go through **explicit API calls** that emit Pulses; the
  Pulses eventually produce Deltas that update the projection.
- **Optimistic UI**: for high-latency actions, apply a speculative
  local delta, then reconcile when the real delta arrives. Label
  speculative state visually (dimmed, italic, or "..." suffix).

A rule that compounds: any piece of UI that needs to live across
page navigation belongs in a projection, not in a page-local store.

## 19. The five-page minimal feature surface

| Page | Subscribed projections | Possible mutations |
|---|---|---|
| Home / Pulse | `cohort_health`, `active_tasks`, `recent_episodes`, `cost_meter`, `alerts` | Acknowledge alert |
| Chat | `agent_trails`, `recent_episodes`, `heuristic_library` (for footnotes) | Send prompt, apply diff, annotate |
| Plans | `plans_list`, `plan_detail/<id>` | Create, pause, resume, execute, reorder tasks |
| Beliefs | `heuristic_library`, `worldview_clusters`, `replication_ledger` | Challenge, retire, edit, import, export |
| Settings | `config_current`, `plugins_list`, `secrets_status` | Set config keys, install/enable plugins, rotate secrets |

Every mutation is an explicit API call. The web UI never reaches
directly into Substrate or Bus — only through the realtime wire
protocol and StateHub projections.

## 20. Responsive / mobile specifics

A phone is not a shrunk desktop. Page-by-page mobile rules:

- **Home**: 5 tiles stack vertically, each full-width. Alerts
  show as a sticky footer if any are pending.
- **Chat**: full-screen; keyboard + voice-input button. Tool-call
  banners collapse by default; expand on tap.
- **Plans**: DAG becomes a vertical list of tasks with expand
  arrows. Drag-to-reorder disabled on mobile; long-press for move.
- **Beliefs**: heuristic cards stack; calibration CI renders as
  mini bar on each card. Worldview clusters scroll horizontally.
- **Settings**: read-only for most fields; explicit "Edit on
  desktop" hint for irreversible operations.

Touch target minimum 44pt (Apple HIG). Spacing generous. Font
at least 16px (browser won't zoom on iOS Safari).

## 21. Server-side rendering and first-paint

SvelteKit defaults to SSR for the first page load. Rules:

- **Home** SSRs the projection initial state; hydrates with live
  subscription client-side. First paint under 500 ms on a good
  connection.
- **Chat, Plans, Beliefs** SSR the shell + auth state; load
  projections after hydration. Keeps bundle size down for landing.
- **Settings** is a protected route; SSR loads auth only; form
  state loads on client.

Budget: Lighthouse Performance >= 90 on Home, >= 85 on others.

## 22. Deep-link semantics

Shareable URLs from §6 are load-bearing. Explicit cases:

- `/plans/<slug>?task=<id>` — pre-opens task focus on the plan page.
- `/chat/<session>?cursor=<c>` — scrolls chat to a specific message.
- `/episodes/<hash>` — direct link into an episode's detail view.
- `/beliefs/h/<id>?challenge=true` — opens challenge modal on the
  heuristic.
- `/replay/<episode>?t=0:45` — opens replay scrubber at a specific
  offset.

When a user shares a deep link:

- Public session links expire (default 24h; configurable).
- Private session links require auth.
- Shared replay links embed a snapshot cursor so stale state
  doesn't resolve differently from when the link was created.

## 23. Cross-references

- Projection layer this UI consumes: `26-statehub-rearchitecture.md`.
- Wire protocol behind subscriptions: `27-realtime-event-surface.md`.
- Component library primitives (diffs, footnotes, scrubbers) come
  from: `30-rich-ux-primitives.md`.
- CLI equivalent of each mutation: `28-cli-parity-familiar-workflows.md`.
- Permission model for mutating actions:
  `32-safety-sandbox-provenance.md` §4.
- Observability for the UI (RUM metrics, error tracking):
  `33-observability-telemetry.md` §7.
- Plugin-contributed custom pages and tiles:
  `17-plugin-extension-architecture.md` §2.

--- END 29-web-ui-architecture.md ---

# Batch REF29 — Web UI five-page architecture across interfaces

**Refinement source**: `tmp/refinements/29-web-ui-architecture.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- `docs/12-interfaces/` — five-page first-party web UI (Home, Chat, Plans, Beliefs, Settings) architecture; SvelteKit + Tailwind; component library.

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/29-web-ui-architecture.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.


## Required vocabulary (verify)

The verify step greps for: `Home|Chat|Plans|Beliefs|Settings|SvelteKit|component library`

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
- Commit ready with message `refinements(REF29): Web UI five-page architecture across interfaces`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
