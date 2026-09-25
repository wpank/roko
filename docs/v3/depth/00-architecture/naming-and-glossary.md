# Naming and Glossary

> **v3 depth file** -- `/docs/v3/depth/00-architecture/naming-and-glossary.md`
> Canonical source: v1 `docs/v1/00-architecture/01-naming-and-glossary.md`
> Status: **Current** -- all terms verified against the shipped codebase as of September 2026

---

## Naming Convention

This is the canonical vocabulary reference for Roko. Use this document when writing docs,
code comments, interfaces, or external material. If another document disagrees, this glossary
wins.

**First-time readers**: this chapter is an A-Z lookup. Start here when another architecture
doc uses a term you do not recognize, then follow the cited home doc for depth.

**Status tags**:
- `[shipping]` = working type, module, or behavior in the current codebase.
- `[built]` = code exists, but the glossary term overstates how fully it is wired.
- `[planned]` = target-state design term with no corresponding shipped type or runtime path yet.
- `[retired]` = historical term deliberately replaced by newer vocabulary.

---

## Current Naming Map

| Canonical term | Status | Use | Avoid |
|---|---|---|---|
| `Roko` | `[shipping]` | Project and framework name | `Bardo` / `Mori` (retired) |
| `Agent` | `[shipping]` | Running process or session | `Golem` (retired) |
| `Signal` | `[shipping]` | Primary protocol noun (`Signal` is the struct; `pub type Engram = Signal` is the backward-compat alias) | Using `Engram` as the public API name |
| `Pulse` | `[shipping]` | Ephemeral transport medium on the Bus | `Event`, `Envelope`, `Message` |
| `Store` | `[shipping]` | Durable storage trait | `Substrate` (kept as blanket-impl alias) |
| `Substrate` | `[shipping]` | Legacy alias trait; `impl<T: Store> Substrate for T {}` | Treating it as separate from Store |
| `ColdStore` | `[shipping]` | Archival storage trait for aged-out Signals | |
| `Bus` | `[shipping]` | Transport trait for Pulses | `EventBus` (internal impl detail) |
| `Neuro` | `[shipping]` | Durable knowledge cross-cut | `Grimoire` (retired) |
| `Daimon` | `[shipping]` | Affect cross-cut; public alias `AffectBias` | |
| `Dreams` | `[shipping]` | Offline consolidation cross-cut | |
| `Cell` | `[shipping]` | Graph execution unit | |
| `Graph` | `[shipping]` | Sole execution engine (DAG of Cells) | `WorkflowEngine` (retired #276) |
| `StateHub` | `[shipping]` | Dashboard/event hub; projection layer | |
| `Fleet` | `[planned]` | Agent roster | `Clade` (retired) |
| `Mesh` | `[planned]` | Agent-network layer | `Styx` (retired) |
| `Topic` | `[shipping]` | Routing handle for Pulse publication | |
| `TopicFilter` | `[shipping]` | Subscription matcher for Bus consumers | |
| `Datum` | `[shipping]` | Polymorphic `Signal` or `Pulse` input enum | |

---

## A

**Active inference** -- Predict-publish-correct loop carried across operators with
`prediction.*`, `outcome.*`, and `prediction.error.*` Pulses.

**ACT** -- Step 4 of the seven-step universal loop: execute the composed Signal as an LLM
call, tool call, or chain call.

**Agent** `[shipping]` -- Running process or session that drives the universal loop end to
end. Formerly `Golem`.

**Attestation** `[built]` -- Cryptographic signature over a Signal's `ContentHash`. The shipped
code has `Attestation`, `ChainAttestation`, and sign/verify support.

**Authorization** `[built]` -- The safety layer authorizes actions through
`SafetyLayer::check_pre_execution()`, `AgentContract`, and `AgentWarrant`.

## B

**Balance** `[built]` -- A knowledge entry's demurrage-taxed attention credit. Starts at
`1.0`, decays over time, and is restored by reinforcement.

**BROADCAST** -- Step 6b of the seven-step loop, co-equal with `PERSIST`: publish Pulses
to the Bus.

**Budget** `[shipping]` -- Constraint struct for Composers: token limit, byte limit, signal
count, wall time.

**Bus** `[shipping]` -- Transport trait for ephemeral Pulses; sibling to `Store`. Implemented
by `PulseBus` in `roko-runtime`.

## C

**c-factor** `[built]` -- Collective-intelligence factor computed continuously from Bus and
Store statistics for agent cohorts. Public alias: **coordination health**.

**CascadeRouter** `[shipping]` -- Bandit-based model router that picks a model per turn.
Persists to `.roko/learn/cascade-router.json`.

**Cell** `[shipping]` -- Graph execution unit. All kernel traits extend `Cell`. Seven
cognitive Cells and five Verify Cells are wired.

**ContentHash** `[shipping]` -- `BLAKE3(kind, body, author, tags)` identifier for a Signal.

**Context** `[shipping]` -- Sidecar state passed to operators; carries the current goal,
session, and workspace configuration.

**ColdStore** `[shipping]` -- Archival storage trait for aged-out Signals. Implemented by
`ArchiveColdSubstrate` in `roko-fs`.

**Custody** `[built]` -- Chain-of-custody audit record for auditable actions.

## D

**Daimon** `[shipping]` -- Affect cross-cut maintaining PAD state, biasing Scorers, and gating
actions. Public alias: **AffectBias**. Wired into per-task runner dispatch.

**Datum** `[shipping]` -- `enum Datum<'a> { Signal(&'a Signal), Pulse(&'a Pulse) }` used by
polymorphic operators.

**Decay** `[shipping]` -- Time-based weight decay enum: `None`, `HalfLife`, `Ttl`,
`Ebbinghaus`. See `decay-variants-and-tier-matrix.md`.

**Dreams** `[shipping]` -- Offline consolidation cross-cut. Resident daemon scheduling is
live for adaptive idle, cron, and episode-count triggers.

## E

**Engram** `[shipping]` -- Backward-compat alias for `Signal`. `pub type Engram = Signal` in
`roko-core/src/engram.rs`. Use `Signal` as the public name.

**Episode** -- Signal kind recording a full agent turn, including inputs, tool calls, outputs,
and verdicts.

## F

**Fingerprint** `[built]` -- HDC fingerprint attached to Signals for similarity queries.
`HdcVector` in `roko-primitives`.

## G

**Gate** -- See `Verify` trait. Connects Roko to external reality: compile, run tests,
simulate transactions, validate schemas. 19 gates in 7-rung pipeline.

**GateVerdict** -- Signal kind produced by a Verify implementation; includes pass/fail,
reason, and evidence.

**Graph** `[shipping]` -- Sole execution engine since #260. DAG of Cells with bounded
parallel waves, conditional routing, and cost enforcement.

## H

**HDC** -- Hyperdimensional Computing: 10,240-bit vectors with bind, bundle, permute,
similarity, and consensus operations. In `roko-primitives`.

**Heartbeat** -- Cognitive clock publishing tick Pulses at Gamma, Theta, and Delta cadence.

## K

**Kernel traits** -- The 12 traits: Store, ColdStore, Score, Verify, Route, Compose, React,
Bus, Observe, Connect, Trigger, Substrate. See `synapse-traits-12.md`.

**Kind** `[shipping]` -- Semantic category enum for Signals and Pulses.

**KnowledgeTier** `[shipping]` -- Tier enum: Transient, Working, Consolidated, Persistent.

## L

**Layer (L0-L4)** -- Five-layer taxonomy: Runtime, Framework, Scaffold, Harness,
Orchestration. See `five-layer-taxonomy.md`.

**Lineage** -- `Vec<ContentHash>` on a Signal pointing to its parents in the durable audit DAG.

## M

**MCP** -- Model Context Protocol for tool integration over stdio or HTTP. Wired in
`roko-agent` and `roko-mcp-*` crates.

## N

**Neuro** `[shipping]` -- Durable knowledge cross-cut covering storage, distillation, and
tier progression. Formerly `Grimoire`. In `roko-neuro`.

## P

**PAD vector** -- Pleasure-Arousal-Dominance affective state maintained by Daimon.

**Plan** -- Signal kind representing a structured multi-task plan with DAG edges.

**Playbook** -- Signal kind storing a distilled reusable action sequence.

**Policy** -- See `React` trait.

**Provenance** `[shipping]` -- Author, trust, taint, and session record on a Signal. See
`provenance-and-attestation.md`.

**Pulse** `[shipping]` -- Ephemeral transport medium on the Bus. Typed, sequence-numbered,
topic-addressed. Lives on a Bus.

## R

**REACT** -- Step 7 of the seven-step loop: `React::decide` emits follow-on Pulses and
Signals.

**Role** -- Composition template plus tool allow-list and gate defaults.

**Router** -- See `Route` trait.

## S

**Score** `[shipping]` -- 7-axis appraisal struct: confidence, novelty, utility, reputation,
precision, salience, coherence. See `score-7-axis-appraisal.md`.

**Signal** `[shipping]` -- The primary protocol noun. The canonical struct in
`roko-core/src/signal.rs`. `pub type Engram = Signal` is the backward-compat alias. Content-addressed, lineage-bearing, scored, and persisted in a Store.

**Store** `[shipping]` -- Durable storage trait. Implementations: `MemorySubstrate`,
`FileSubstrate`. See `substrate-trait.md`.

**StateHub** `[shipping]` -- Push-based dashboard event hub. DashboardEvent -> watch::Sender
-> TUI/SSE/WS.

## T

**Taint** `[shipping]` -- Trust-origin metadata on Signals. The shipped code implements a
trust-origin IFC lattice with TaintTracker.

**Topic** `[shipping]` -- Routing handle for Pulses. Dot-separated lowercase strings such as
`gate.verdict.emitted`.

**TopicFilter** `[shipping]` -- Declarative subscription matcher with variants: `Exact`,
`Glob`, `AnyOf`, `All`, `And`, `Or`, `Not`.

## V

**Verify** `[shipping]` -- Kernel trait for verification gates. Async by default.

**Verdict** `[shipping]` -- Output of a Verify implementation, persisted as a GateVerdict
Signal.

---

## Retired Terms

| Old | Replaced by | Reason |
|---|---|---|
| `Bardo`, `Mori` | `Roko` | Retired project codenames |
| `Golem` | `Agent` | Retired runtime-entity name |
| `Grimoire` | `Neuro` | Retired knowledge-cross-cut name |
| `Styx` | `Mesh` + `Korai` | One umbrella split into two concepts |
| `Clade` | `Fleet` | Fleet is the conventional roster term |
| `WorkflowEngine` | `Graph` | Retired by #276 |
| Death/mortal/reincarnation framing | Remove | Use custody, export/import, resource, budget language |

---

## Terms Deliberately Not Defined

Some words still use ordinary engineering meaning rather than a formal Roko-specific
definition:

- `session` in the OIDC or HTTP sense
- `task` in the general async-runtime sense
- `model` when the text clearly means an LLM
- `cost` when the text simply means currency spend

---

## Maintenance

- Every new technical term introduced in an architecture doc should add a glossary entry in
  the same change.
- Retiring a term moves it into the retired table with a reason.
- Cross-references elsewhere in `docs/` should use the spellings in this chapter.
- Review this chapter whenever a new primitive, cross-cut, interface surface, or safety
  concept becomes load-bearing.
