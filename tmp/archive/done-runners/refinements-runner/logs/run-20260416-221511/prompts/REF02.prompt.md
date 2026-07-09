# Refinements Batch REF02

Run id: run-20260416-221511
Attempt: 2
Model: gpt-5.4
Reasoning: high
Refinement source: /Users/will/dev/nunchi/roko/roko/tmp/refinements/02-engram-vs-pulse.md
Target docs (candidates): docs/00-architecture/02-engram-data-type.md docs/00-architecture/INDEX.md docs/00-architecture/01-naming-and-glossary.md

## Previous attempt failure context

Terminology gate failed: retired terms present in changed files.

Recent log tail:
=== Duration: 6m 18s ===
=== Exit code: 0 ===
[verify] diff_gate: 5 changed path(s) under docs/
[verify] terminology: scanning 5 file(s)
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 23:| Bardo | **Roko** | Overall framework/project name. "Bardo" was the umbrella name for the original ecosystem. |
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 55:| `bardo-terminal` | `roko-cli` | The terminal UI is in `roko-cli`, with a separate TUI scaffold. |
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 102:| Bardo Sanctum | **Roko Portal** (web dashboard) |
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 103:| bardo-terminal / Bardo | **Roko TUI** (terminal dashboard) |
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 309:5. **File paths**: All `bardo-*` → `roko-*`, all `mori-*` → `roko-*`.
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 25:| Golem / Golems | **Agent / Agents** | The autonomous entity. "Agent" is the generic term; "Roko agent" when disambiguation is needed. The framework is Roko; individual entities are agents. |
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 42:| `golem.toml` | `roko.toml` |
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 50:| All `golem-*` | `roko-*` | Mechanical rename. |
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 56:| `roko-golem` | **DISSOLVED** | See Crate Dissolution section below. |
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 60:## 4. Crate Dissolution: `roko-golem`
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 62:The `roko-golem` crate has been dissolved. Its subsystems are redistributed to standalone
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 68:| Daimon (972 lines, fully implemented) | `roko-golem/daimon.rs` | `roko-daimon` | Full PAD vector implementation, behavioral states, somatic markers. |
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 69:| Dreams (43 lines, placeholder) | `roko-golem/dreams.rs` | `roko-dreams` | Placeholder deleted; `roko-dreams` is the expanded implementation. |
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 70:| Grimoire (44 lines, placeholder) | `roko-golem/grimoire.rs` | `roko-neuro` | Placeholder deleted; `roko-neuro` is the replacement with tier-based knowledge management. |
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 71:| Chain Witness (43 lines, placeholder) | `roko-golem/chain_witness.rs` | `roko-chain` as `chain_witness` module | Moved. |
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 72:| Mortality (44 lines, placeholder) | `roko-golem/mortality.rs` | **DELETED ENTIRELY** | No mortality system in the new architecture. Resource constraints (budget, confidence, time) replace mortality clocks. |
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 73:| Hypnagogia (42 lines, placeholder) | `roko-golem/hypnagogia.rs` | `roko-dreams` as `hypnagogia` module | Moved to the Dreams crate. |
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 74:| `ScaffoldEngine` trait | `roko-golem/lib.rs` | **DELETED** | Each subsystem defines its own trait. No umbrella needed. |
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 75:| `GolemScaffold` aggregator | `roko-golem/lib.rs` | **DELETED** | Composition happens at the application layer via configuration. |
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 77:After dissolution, `roko-golem` is removed from workspace members in `Cargo.toml`.
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 116:"DAEJI token" if explicitly about testnet). When it mentions "golem chain," the correct
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 310:6. **Never say**: "Golem SDK" → say "Agent SDK" or "Roko SDK".
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 311:7. **Never say**: "Mori + Golem" → say "Roko framework with coding and chain domain plugins."
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 24:| Mori | **Roko Orchestrator** | Build/coding orchestration. Often just "orchestrator." The original Mori was a 108K-LOC TypeScript/Rust application for coding agent orchestration. |
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 52:| `mori-index` | `roko-index` | Code parsing, symbol graphs, HDC fingerprints. |
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 53:| `mori-context` | `roko-compose` + `roko-index` | Context features moved to `roko-compose`; code intelligence moved to `roko-index`. |
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 54:| `mori-mcp` | `roko-mcp-{stdio,github,slack,scripts}` | Split into per-transport MCP server crates. |
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 104:| Mori TUI | **Roko TUI** |
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 309:5. **File paths**: All `bardo-*` → `roko-*`, all `mori-*` → `roko-*`.
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 311:7. **Never say**: "Mori + Golem" → say "Roko framework with coding and chain domain plugins."
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 26:| Grimoire | **Neuro** / `roko-neuro` / **NeuroStore** | Knowledge management subsystem. Persists insights, heuristics, warnings with tier-based decay. |
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 70:| Grimoire (44 lines, placeholder) | `roko-golem/grimoire.rs` | `roko-neuro` | Placeholder deleted; `roko-neuro` is the replacement with tier-based knowledge management. |
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 27:| Styx | **Agent Mesh** / **Mesh** | P2P relay and permissioned subnets for inter-agent communication and knowledge sharing. |
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 187:| **Agent Mesh** | P2P communication and knowledge sharing between Roko agents. Replaces "Styx." |
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 31:| Clade | **Collective** / **Mesh** | Groups of cooperating agents. **Never use "fleet"** — this was an incorrect interim rename. |
[verify] terminology violation in docs/00-architecture/01-naming-and-glossary.md: 205:| **Collective** | A group of cooperating agents. Replaces "Clade." |

Use that context to avoid repeating the same failure.

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

Suggested parallel split for batch `REF02`:

- worker: extend `docs/00-architecture/02-engram-data-type.md` with a Pulse
  sibling section and a link forward to 02b (if split); remove stale "Signal"
  disclaimers.
- worker: add a new `docs/00-architecture/02b-pulse-ephemeral-event.md` (or
  add a full section to 02) covering Pulse type, conversion law, and graduation
  policy.
- worker: update `docs/00-architecture/01-naming-and-glossary.md` with Pulse +
  related terms (Topic, TopicFilter, PulseSource).

## Canonical refinement source

This is the verbatim refinement proposal that this batch must propagate
into `docs/`. Treat it as the authoritative source. Do not edit this
file; only edit docs under `docs/`.

--- BEGIN 02-engram-vs-pulse.md ---

# Two Mediums: Engram (Durable) and Pulse (Ephemeral)

> **TL;DR**: Keep the Engram exactly as it is — the content-addressed,
> lineage-bearing, decayed, provenance-stamped record. Introduce a sibling
> type, **Pulse**, for the in-flight message. Define a clean conversion
> law so Pulses can graduate into Engrams when their lineage matters.

> **For first-time readers**: An **Engram** today is a Roko Rust struct —
> hashed by BLAKE3 over its `kind`/`body`/`author`/`tags`, decayed on a schedule,
> scored along 7 axes, chained by lineage to parent Engrams. It is Roko's one
> existing data type. A **Pulse** (this proposal) is its sibling: a typed,
> sequence-numbered, brief message that lives in an event-bus ring buffer
> and delivers once. This doc names the Pulse, lists which fields it has and
> doesn't, and defines when a Pulse should "graduate" into an Engram.

## 1. The split

| Property | Engram | Pulse |
|---|---|---|
| Identity | `ContentHash` (BLAKE3 over kind + body + author + tags) | `(topic, seq)` within a Bus; no global hash |
| Durability | Persisted in a `Substrate` | Lives in a Bus ring buffer; drops when ring wraps |
| Lineage | `Vec<ContentHash>` — audit DAG | Optional `lineage_hint: Option<ContentHash>` pointing at an Engram |
| Decay | `Decay` enum (HalfLife, Ttl, Ebbinghaus, None) | N/A — Pulses are instantaneous |
| Score | `Score` (7-axis appraisal) | N/A — Pulses may be scored in flight but don't carry a score |
| Provenance | Full `Provenance` (author, trust, taint, attestation) | Lightweight: `source: String`, topic implies author class |
| Attestation | Optional Ed25519 / chain attestation | None |
| Typical rate | 1 Hz – 1 kHz (plans, tasks, verdicts, episodes) | 1 Hz – 1 MHz (heartbeats, tokens, stream chunks) |
| Typical consumer | Scorer, Gate, Composer, Substrate | Policy, TUI, HTTP subscribers, sidecar, dashboards |
| Examples | Plan, Task, AgentOutput (final), GateVerdict, Episode, Playbook, Insight, Heuristic, Pheromone, Prediction, Attestation | ProcessSpawn, ProcessExit, AgentMessage chunk, TokenUsage tick, ApprovalRequested, GateVerdictInFlight, ContextUpdated, HeartbeatTick, CancellationRequested, UiRefresh |

The split is *not* between important and unimportant data. It's between
**data that needs to be auditable forever** and **data that needs to be
delivered right now and maybe remembered briefly**.

## 2. The Pulse type

Proposed shape, mirroring `Envelope<E>` in `roko-runtime` but canonicalized:

```rust
/// An in-flight event traveling on a Bus.
///
/// Pulses are typed, sequence-numbered, timestamped messages. They are
/// not content-addressed and are not persisted by default. A Pulse may
/// carry an optional lineage hint pointing at an Engram whose
/// ContentHash contextualizes it.
///
/// Pulses may be "graduated" to Engrams by a Policy via
/// `Pulse::graduate(provenance, decay) -> Engram`. This is the ONLY
/// path from transport into audit-DAG.
#[derive(Clone, Debug)]
pub struct Pulse {
    /// Topic-local monotonic sequence. Unique per (bus, topic) pair.
    pub seq: u64,
    /// Topic string, e.g. "gate.verdict" or "agent.msg.chunk".
    pub topic: Topic,
    /// Kind — reused from Engram. Same taxonomy.
    pub kind: Kind,
    /// Payload — reused from Engram. Same taxonomy.
    pub body: Body,
    /// Unix milliseconds when the Pulse was published.
    pub emitted_at_ms: i64,
    /// Lightweight source attribution (component name, agent id, etc.).
    pub source: PulseSource,
    /// Optional Engram reference that gives context for this Pulse.
    /// E.g. an AgentMessage Pulse may reference the Task Engram it
    /// belongs to.
    pub lineage_hint: Option<ContentHash>,
    /// Optional trace id for distributed tracing. Not part of identity.
    pub trace_id: Option<TraceId>,
}
```

### 2.1 Topics

Topics are strings with a recommended hierarchy, like OpenTelemetry
span names:

```
orchestration.plan.started
orchestration.task.ready
agent.msg.chunk
agent.process.spawned
agent.process.exited
agent.tokens.used
gate.verdict.emitted
gate.pipeline.failed
safety.approval.requested
safety.taint.propagated
conductor.circuit.tripped
conductor.health.degraded
ui.refresh.requested
chain.transaction.confirmed    (Phase 2+)
mesh.pheromone.deposited       (Phase 2+)
```

Topics are the contract surface between subsystems. They replace today's
ad-hoc `OrchestrationEvent`, `AgentEvent`, `UiEvent` enums with a single
string-namespaced taxonomy.

### 2.2 Why reuse `Kind` and `Body`?

The Engram's `Kind` enum in `crates/roko-core/src/kind.rs` already
enumerates the semantic categories of the system (ProcessSpawn,
AgentMessage, GateVerdict, TokenUsage, …). Reusing it for Pulses means a
Pulse and an Engram that describe the same event have the same `kind`
and `body`, which makes graduation trivially an identity function plus
some extra fields.

This also means existing code that dispatches on `Kind` continues to
work: a Policy that reacts to `Kind::GateVerdict` Pulses is the
*same* Policy that reads `Kind::GateVerdict` Engrams from storage
during replay.

## 3. The conversion law

Graduation is the well-defined path from Pulse to Engram:

```rust
impl Pulse {
    /// Graduate this Pulse into an Engram suitable for Substrate storage.
    ///
    /// The caller supplies provenance (author, trust, taint chain) and
    /// a decay policy. Lineage is carried forward from `lineage_hint`.
    ///
    /// The resulting Engram's ContentHash is computed from
    /// (kind, body, provenance.author, tags), as usual. If two Pulses
    /// graduate with identical content they produce identical
    /// Engrams — deduplication is automatic.
    pub fn graduate(
        &self,
        provenance: Provenance,
        decay: Decay,
        score: Score,
        tags: BTreeMap<String, String>,
    ) -> Engram {
        let lineage = self.lineage_hint.clone().into_iter().collect();
        EngramBuilder::new(self.kind.clone(), self.body.clone())
            .created_at_ms(self.emitted_at_ms)
            .provenance(provenance)
            .decay(decay)
            .score(score)
            .lineage(lineage)
            .tags(tags)
            .build()
    }
}
```

The reverse — Engram to Pulse — is a lossy projection (loses score,
decay, lineage vector):

```rust
impl Engram {
    /// Project this Engram onto a Pulse for broadcast.
    ///
    /// Used when a stored Engram needs to be announced to live
    /// subscribers (e.g. replay-on-resume, or dashboard updates).
    pub fn to_pulse(&self, topic: Topic, seq: u64, source: PulseSource) -> Pulse {
        Pulse {
            seq,
            topic,
            kind: self.kind.clone(),
            body: self.body.clone(),
            emitted_at_ms: self.created_at_ms,
            source,
            lineage_hint: Some(self.id.clone()),
            trace_id: None,
        }
    }
}
```

### 3.1 Graduation policy

**Not every Pulse should graduate.** Heartbeat ticks, UI refresh
requests, and intermediate token-usage samples have no lineage value
and should die in the ring buffer. Good defaults:

| Pulse topic | Graduate? | Reason |
|---|---|---|
| `orchestration.plan.started` | Yes | Plan lifecycle belongs in DAG |
| `orchestration.task.ready` | No | Redundant with Task Engram already in Substrate |
| `agent.msg.chunk` | Batch-graduate on stream close | Individual chunks are noise |
| `agent.process.spawned/exited` | Yes | Process lifecycle is forensic |
| `agent.tokens.used` | Aggregate then graduate | Per-chunk is noise; per-turn is useful |
| `gate.verdict.emitted` | Yes | Verdicts are the core audit record |
| `safety.approval.requested` | Yes | Safety events must be auditable |
| `conductor.circuit.tripped` | Yes | Health events are forensic |
| `ui.refresh.requested` | No | UI-local |
| `heartbeat.tick` | No | Clock pulses are infrastructure |

This table itself should live in `roko-core` as a `GraduationPolicy`
default implementation, overridable via config.

## 4. Why this split is safer than it looks

### 4.1 It matches what the code already does

`roko-agent-server` already publishes WebSocket token-chunk events that
are not Engrams. `roko-orchestrator` already emits `OrchestrationEvent`
on a bus. `roko-runtime` already defines `Envelope<E>`. We are
*naming* what exists, not inventing a second type system.

### 4.2 It preserves the Engram invariants

The forensic AI capability and the content-addressed DAG depend on
Engrams being *exactly* what they are today: hashed, lineage-bearing,
decayed. Pulses don't weaken this — they just stop forcing ephemeral
events into the hashed DAG when they don't need to be there.

### 4.3 It lets the Substrate remain the only persistence surface

Pulses aren't persisted. If a Pulse matters long-term, it graduates to
an Engram and goes to the Substrate. There is still exactly one
storage surface, with one audit model. We added a transport surface
alongside it, which is what we have been calling the event bus all
along.

### 4.4 It doesn't break Policy

Policy's signature changes from:

```rust
fn decide(&self, stream: &[Engram], ctx: &Context) -> Vec<Engram>;
```

to:

```rust
fn decide(&self, stream: &[Pulse], ctx: &Context) -> PolicyOutputs;
// where PolicyOutputs = { pulses: Vec<Pulse>, engrams: Vec<Engram> }
```

Existing Policy implementations that were doing
`Policy::decide(&[], ctx)` with synthetic empty Engram streams
(doc 23 calls this "awkward but functional") can now emit their
metric Pulses cleanly. Policies that want to react to stored Engrams
can still do so — they subscribe to a `substrate.*` topic that the
Substrate emits when Engrams land.

## 5. Worked example — agent turn

Current state, one-noun model:

1. Agent subprocess spawns → ad-hoc `AgentEvent::ProcessSpawned` on bus.
2. Token chunks arrive → ad-hoc `AgentEvent::StreamChunk` on bus.
3. Turn completes → `Engram { kind: AgentOutput, body: Text(...) }`
   written to Substrate.
4. GatePipeline verifies the Engram → `Engram { kind: GateVerdict }`
   written to Substrate.
5. TUI polls Substrate for new Engrams (doc 24 flags this as the
   P0 "polling-vs-streaming" bug).

Two-medium model:

1. Agent subprocess spawns → publish `Pulse { topic:
   "agent.process.spawned", kind: ProcessSpawn, ... }`. Policy
   graduates it to an Engram (process lifecycle is forensic).
2. Token chunks arrive → publish `Pulse { topic: "agent.msg.chunk", ... }`
   at 10–100 Hz. Not graduated. TUI subscribes, renders incrementally.
3. Turn completes → publish `Pulse { topic: "agent.turn.completed", ... }`
   AND graduate to `Engram { kind: AgentOutput }` written to
   Substrate.
4. GatePipeline runs → publishes `Pulse { topic:
   "gate.verdict.emitted", ... }` AND graduates to
   `Engram { kind: GateVerdict }`.
5. TUI never polls — it subscribes to `gate.verdict.*`,
   `agent.msg.*`, `orchestration.*` topics. The P0 bug dissolves.

The agent turn is the same turn. We just stopped forcing every
heartbeat and token into the hashed DAG, and we stopped making the TUI
poll a database.

## 6. Things this does not answer

- **Does Pulse need a hash at all?** For replay determinism across a
  restart, maybe a lightweight non-content-addressed id helps. Or we
  accept that replay is best-effort until a Pulse graduates. See
  `09-phase-2-implications.md` for the chain-replay implications.
- **What about backpressure?** Pulses are broadcast, not queued. If a
  subscriber is slow, it misses. This is the same model `tokio::sync::broadcast`
  uses. For critical subscribers, they can graduate the Pulse and
  subscribe to the Substrate.
- **Who owns topic names?** They need a registry. A `roko-core::topics`
  module with `const TOPIC_AGENT_MSG_CHUNK: &str = "agent.msg.chunk"`
  declarations is the minimum; a richer `Topic` newtype with validation
  is better. `07-naming.md` §4 proposes the file layout for this.
- **How large should Pulse bodies get?** Small is good: Pulses fan out
  to all subscribers, so cost is *O(body × subscribers)*. A 5 MB token
  stream slice on a topic with 50 subscribers is 250 MB of copies per
  publish. Rule of thumb: bodies under 64 KB for hot topics (`agent.msg.chunk`),
  under 1 MB for structural topics (`orchestration.plan.started`). If it
  has to be bigger, put the payload in a Substrate Engram and let the
  Pulse carry only a `lineage_hint` pointing at it.

## 7. What else changes if Pulse lands

Adopting Pulse ripples through four other proposals in this folder. Keep
these in mind when reviewing:

- **`10-self-learning-cybernetic-loops.md`** — every operator gets a
  prediction/outcome Pulse pair, which only works if Pulses are a
  first-class type. Without Pulse, active inference has nowhere to publish
  the prediction-error signal.
- **`12-knowledge-demurrage.md`** §2 — the `ReinforceKind::Retrieved` and
  `ReinforceKind::Surprised` signals ride on Pulses. Demurrage without
  Pulse either forces every read to write a new Engram (expensive) or
  skips reinforcement entirely (misses the point).
- **`13-collective-intelligence-c-factor.md`** §2.2 — every c-factor
  metric is computable from Bus statistics. Authorship entropy, delivery
  rate, peer-prediction accuracy — all are Pulse-level observations.
- **`26-statehub-rearchitecture.md`** — StateHub projections consume Bus
  subscriptions to build their live views. Without a typed Pulse the
  projection layer has to carry subsystem-specific enums through a
  generic bus, which is what `Envelope<E>` does today and why it's ad hoc.

## 8. Before / after cheat sheet

The same five lines of code, once without Pulse and once with:

```rust
// Before: ad hoc, per-crate enum lives on a generic broadcast channel.
use roko_orchestrator::OrchestrationEvent;
tx.send(OrchestrationEvent::PlanStarted { plan_id, started_at_ms });
```

```rust
// After: one Pulse shape, topic-addressed, contextually graduatable.
use roko_core::{Pulse, Topic, Kind, Body, PulseSource};
bus.publish(Pulse {
    seq: 0,                              // filled by bus at publish time
    topic: Topic::new("orchestration.plan.started"),
    kind: Kind::Plan,
    body: Body::Json(json!({ "plan_id": plan_id, "started_at_ms": started_at_ms })),
    emitted_at_ms: now_ms(),
    source: PulseSource { component: "roko-orchestrator".into(), agent_id: None },
    lineage_hint: Some(plan_hash),
    trace_id: None,
}).await?;
```

The "after" form is longer — but the verbosity is paying for four concrete
things: every subsystem in the workspace speaks the same vocabulary; the
TUI/web/Slack bot can subscribe without importing the orchestrator; demurrage
can reinforce the referenced plan_hash; and graduation is a two-line call
away when the Pulse should become a durable record.

See `07-naming.md` for whether to call this type `Pulse`, `Event`, or
reclaim `Signal`. See `08-code-sketches.md` for the `Pulse::graduate`
implementation and a full end-to-end test.

--- END 02-engram-vs-pulse.md ---

# Batch REF02 — Introduce Pulse (ephemeral medium) across architecture chapter

**Refinement source**: `tmp/refinements/02-engram-vs-pulse.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- `docs/00-architecture/02-engram-data-type.md` — extend with Pulse sibling section, graduation law, conversion helpers. Remove "Signal is the same as Engram" disclaimers.
- `docs/00-architecture/01-naming-and-glossary.md` — add Pulse, Topic, TopicFilter, PulseSource, Datum entries.
- `docs/00-architecture/INDEX.md` — add Pulse to the two-medium abstract.
- Consider adding `docs/00-architecture/02b-pulse-ephemeral-event.md` as a sibling chapter if the content warrants a full file.

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/02-engram-vs-pulse.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.
- Graduation law (Pulse → Engram) documented with provenance upgrade.
- Graduation policy table (which topics graduate, which don't) included in either the 02 file or 02b.

## Required vocabulary (verify)

The verify step greps for: `Pulse`

At least one changed file must mention it (case-insensitive) before the
batch is considered complete.

## Cross-references

Dependents — downstream refinements assuming this one has propagated
into docs: REF03, REF04, REF07, REF11, REF12

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
- Commit ready with message `refinements(REF02): Introduce Pulse (ephemeral medium) across architecture chapter`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
