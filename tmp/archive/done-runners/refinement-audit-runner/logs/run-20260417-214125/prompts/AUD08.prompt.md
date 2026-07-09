# Refinement Audit Runner — Batch AUD08

Run id: run-20260417-214125
Attempt: 1
Model: gpt-5.4
Reasoning: high

## Shared Context Pack

### 00-AUDIT-RULES

# Audit Application Rules

You are applying refinement-audit critiques to Roko's documentation and tooling.
The audit found that the refinements were "directionally correct but 5-10x overscoped."

## Core Principles

1. **The diagnosis is correct, the prescription was overscoped.** Ship what matters.
2. **Split "exists" from "planned."** Never describe unbuilt features in present tense.
3. **Narrow, don't delete.** Move overscoped content to "future work" sections.
4. **Fix factual errors.** Update LOC counts, route counts, crate counts, status labels.
5. **Reduce jargon inflation.** If a concept has 0 lines of code, it's a research hypothesis.

## Verdicts to Apply

- `keep` → Polish wording. Strengthen evidence. Keep it.
- `narrow` → Reduce scope. Add "aspirational" or "target-state" caveats.
- `defer` → Move to explicit future-work section with a clear label.
- `rewrite` → Reframe per the audit's specific guidance. Don't just edit — rethink.

## Factual Corrections (from codebase reality check)

- Total Rust LOC: 322,088 (not 177K)
- Workspace members: 36 (not 18)
- roko-serve routes: 200+ (not ~85)
- TUI: 58K LOC (wired, not "text-mode only")
- roko-learn: 42 modules, 35,847 LOC
- Event bus event types: exactly 2 (PlanRevision, PrdPublished)
- Pulse/Datum/Demurrage/Worldview/Custody: 0 lines of code each

## 5 Aspirational Concepts with 0 Code

These MUST be labeled as "target-state" or "planned" in docs, never described as existing:
1. Pulse (ephemeral event type)
2. Datum (medium polymorphism enum)
3. Demurrage (knowledge decay economic model)
4. Worldview (heuristic cluster)
5. Custody (chain-of-custody record)

### 01-PRIORITY-QUEUE

# Priority Queue

From the audit master summary — this is the recommended priority order.

## Ship Now (1-2 weeks total)

1. Add HDC fingerprint field to Engram — `roko-core/src/engram.rs` — 1 day
2. Unify event enums into `RokoEvent` — across 4 crates — 1 week
3. Add generic `Bus<E>` trait to roko-core — ~100 lines — 2-3 days
4. Clean up stale "Signal" references — traits.rs, README, kind.rs — 1 hour
5. Fix architecture INDEX status — `docs/00-architecture/INDEX.md` — 30 min

## Ship Soon (next month)

6. CLI parity / muscle memory (REF28)
7. StateHub hardening (REF26)
8. Heuristic calibration struct (REF14)
9. Safety: extend Attestation + expand taint (REF32)
10. Threat model doc (REF32 §13)

## Defer

- Pulse type, Datum enum, Operator generalization
- Demurrage, Plugin SPI tiers 4-5, 3 new kernel crates
- All 5 rewrite candidates, SvelteKit web UI, gRPC
- 12-month roadmap timeline

## Wrong (needs correction in docs)

- Synergy matrix (7/10 primitives don't exist)
- REF32 ignores existing safety system
- Glossary marks EventBus as "retired" (it's the only live transport)
- "Moat" framing (2/10 components exist fully)
- Doc INDEX says serve/TUI "not wired" (both definitively wired)

### 02-DOCS-TREE-MAP

# Docs Tree Map

The canonical documentation lives at `docs/`. Here is the full structure:

```
docs/
├── 00-architecture/        # 33+ files; kernel + trait system + analysis + design principles
├── 01-orchestration/       # Plan DAG, execution, plan runner
├── 02-agents/              # Agent dispatch, backends, sidecar
├── 03-composition/         # Prompts, context assembly, templates, budgets
├── 04-verification/        # Gates, validation, 7-rung pipeline
├── 05-learning/            # Self-learning loops, episodes, playbooks, experiments
├── 06-neuro/               # HDC, knowledge store, distillation, tier progression
├── 07-conductor/           # Event watchers, circuit breaker, diagnosis
├── 08-chain/               # On-chain primitives, ChainBus (Phase 2+)
├── 09-daimon/              # Behavior primitives (Phase 2+)
├── 10-dreams/              # Sleep-time compute, consolidation (Phase 2+)
├── 11-safety/              # Role auth, provenance, attestation, taint
├── 12-interfaces/          # CLI, HTTP API, TUI, Web UI, chat
├── 13-coordination/        # Stigmergy, coordination theory, c-factor
├── 14-identity-economy/    # Identity, economic models
├── 15-code-intelligence/   # Parser, indexing, HDC graphs
├── 16-heartbeat/           # Reactive/reflective loops, timing, CoALA mapping
├── 17-lifecycle/           # Agent lifecycle, shutdown
├── 18-tools/               # Tool system, plugin SPI
├── 19-deployment/          # Containers, orchestration, observability
├── 20-technical-analysis/  # Architecture audit, moat analysis, innovations
├── 21-references/          # Bibliography, research papers
├── INDEX.md                # Top-level index
├── STATUS.md               # Current wiring status
├── BENCHMARKS.md           # Performance data
└── CLI-REFERENCE.md        # Command documentation
```

## Key files you'll likely need to edit

- `docs/00-architecture/INDEX.md` — master architecture index (stale status claims)
- `docs/00-architecture/01-naming-and-glossary.md` — canonical glossary
- `docs/00-architecture/15-crate-map.md` — crate dependency graph
- `docs/00-architecture/31-implementation-readiness-audit.md` — readiness status
- `docs/INDEX.md` — top-level doc index
- `docs/STATUS.md` — current wiring status table

## What the refinements-runner already changed

The first pass (`tmp/refinements-runner/`) landed 35 batches (REF01-REF35) that introduced
new concepts (Pulse, Bus, Datum, demurrage, etc.) into the docs. Many of these concepts
have ZERO lines of code. The audit found that the docs now describe aspirational
architecture as if it exists. Your job is to fix that.

### 03-WORKSPACE-TOPOLOGY

# Workspace Topology

Roko is a Rust workspace at `/Users/will/dev/nunchi/roko/roko/`.

## Crate map (36 workspace members)

| Crate | Path | LOC | Status |
|---|---|---|---|
| roko-core | `crates/roko-core/` | kernel | Stable — Engram + 6 traits + config + tools |
| roko-agent | `crates/roko-agent/` | large | 8 LLM backends, pools, MCP, tool loop, safety |
| roko-agent-server | `crates/roko-agent-server/` | medium | Per-agent HTTP sidecar, real LLM dispatch |
| roko-serve | `crates/roko-serve/` | 30K | HTTP control plane, 200+ routes, SSE, WebSocket |
| roko-orchestrator | `crates/roko-orchestrator/` | medium | Plan DAG, parallel executor, merge queue |
| roko-gate | `crates/roko-gate/` | medium | 11 gates, 7-rung pipeline, adaptive thresholds |
| roko-compose | `crates/roko-compose/` | medium | Prompt assembly, 9 templates, enrichment |
| roko-conductor | `crates/roko-conductor/` | medium | 10 watchers, circuit breaker, diagnosis |
| roko-learn | `crates/roko-learn/` | 36K | 42 modules: episodes, playbooks, bandits, routing, experiments |
| roko-cli | `crates/roko-cli/` | 17K+ | CLI binary + ratatui TUI (58K LOC total) |
| roko-fs | `crates/roko-fs/` | small | FileSubstrate (JSONL), GC, layout |
| roko-std | `crates/roko-std/` | medium | Defaults, 19 builtin tools, mock dispatcher |
| roko-runtime | `crates/roko-runtime/` | medium | ProcessSupervisor, event bus, cancellation |
| roko-primitives | `crates/roko-primitives/` | small | HDC vectors (10,240-bit), tier routing |
| roko-neuro | `crates/roko-neuro/` | medium | Durable knowledge store, distillation, tiers |
| roko-mcp-code | `crates/roko-mcp-code/` | medium | Code-intelligence MCP server |
| roko-index | `crates/roko-index/` | medium | Parser + graph + HDC indexing |
| roko-lang-* | `crates/roko-lang-*/` | small | Language support (rust, typescript, go) |
| roko-dreams | `crates/roko-dreams/` | small | Offline consolidation (Phase 2+) |
| roko-daimon | `crates/roko-daimon/` | small | Behavior primitives (Phase 2+) |
| roko-chain | `crates/roko-chain/` | small | Chain witness primitives (Phase 2+) |

## Key numbers (from codebase audit)

- Total Rust LOC: 322,088
- Workspace members: 36
- Test functions: 3,761
- orchestrate.rs: 17,087 lines
- Event bus event types: exactly 2 (PlanRevision, PrdPublished)
- Signal→Engram rename: 99.6% complete

## Concepts with 0 lines of code

These exist ONLY in docs, not in any crate:
- Pulse, Datum, Demurrage, Worldview, Custody
- roko-bus, roko-hdc (as separate crate), roko-spi
- Bus trait (as a formalized kernel trait)

### 04-DELEGATION-GUIDANCE

# Delegation Guidance

You are explicitly authorized to use multiple subagents for this batch.
Use them where it helps, but keep the immediate blocking work local.

## Required delegation behavior

- Before editing, form a short plan and identify 2-4 concrete subtasks.
- Spawn explorers for targeted codebase/docs reads and workers for bounded edits.
- Give each worker a disjoint write scope — no two workers edit the same file.
- Do not wait idly for subagents if you can progress locally.
- If subagents are unavailable in this environment, continue locally without failing.

## Reading files

Before editing any file, READ IT FIRST. You are working in a git worktree
that contains the full repository. Use your file-reading capabilities to
inspect the current state of any file before modifying it.

## Phase-specific guidance

### Phase 1 (AUD* batches) — Docs only
- Only edit files under `docs/`. Never touch `crates/`, `tmp/`, or `src/`.
- Read the target docs before editing to understand their current state.
- The refinements-runner already made changes — you are refining those changes.

### Phase 2 (PU* batches) — Parity content refresh
- Only edit files under `tmp/docs-parity/NN/`.
- Read the current `docs/` tree first to understand what the audit pass changed.
- Update context-pack/, BATCHES.md, 00-INDEX.md, and all batch detail .md files.
- Update the run-docs-parity.sh script if its batch descriptions or verify
  commands reference stale content.

### Phase 3 (PE* batches) — Code execution
- Edit files under `crates/` to implement what the parity docs describe.
- Read BATCHES.md and 00-INDEX.md from the parity section FIRST.
- Search before writing: `grep -rn 'Name' crates/ --include='*.rs' | grep -v target/`
- Wire existing code — do not reimplement what already exists.
- Run `cargo check` after changes to verify compilation.

## Audit Source Files

These are the critique/triage documents that drive your edits.
Read them carefully — they contain specific verdicts (keep/narrow/defer/rewrite)
and codebase reality checks.

--- BEGIN 07-naming-and-term-cuts.md ---

# Naming And Term Cuts

This file proposes cleaner names and clearer scope boundaries for the terms most
likely to confuse users or overcomplicate the redesign.

## Naming principles

Prefer names that:
- reveal which lane a concept belongs to;
- are legible to a new engineer without a glossary deep-dive;
- do not force a theoretical worldview into every API;
- can survive contact with CLI, UI, logs, and code.

## Keep

| Term | Decision | Why |
|---|---|---|
| `Engram` | keep | Distinct, stable, and well-matched to durable records. |
| `Pulse` | keep | Best ephemeral transport noun in the set. |
| `Bus` | keep | Standard transport term with low ambiguity. |
| `Substrate` | keep | Still works as the storage fabric term. |
| `Topic` | keep | Familiar routing handle; just validate it. |
| `Projection` | elevate | This should become the main public read-model term. |
| `Session` | elevate | More intuitive user-facing object than many lower-level terms. |

## Keep, but narrow or reposition

| Term | Decision | Recommended role |
|---|---|---|
| `StateHub` | keep, but demote | Runtime host for projections, not the main public abstraction. |
| `Custody` | keep, but narrow | Safety/audit term only, not a general runtime noun. |
| `Domain profile` | keep | Good packaging term for bundles of tools, roles, and defaults. |
| `Runtime shape` | add | Use this for laptop/server/container/cluster to avoid overloading `profile`. |

## Split

| Current term | Proposed split | Why |
|---|---|---|
| `Policy` | `Policy` + `Calibrator` | Control and learning should not be one concept. |
| `Heuristic` | `HeuristicSpec` + `Calibration` | Rule and evidence history evolve differently. |
| `Profile` | `domain profile` + `runtime shape` | One word is doing too much work today. |
| `StateHub` | `Projection` + `StateHub` host | Public concept and implementation host should not be fused. |

## Replace in public-facing docs

| Current term | Better term | Why |
|---|---|---|
| `TypedContext` | `Situation` | More intuitive for users and operators. |
| `c-factor` | `coordination health` | Says what it means without prior literature knowledge. |
| `worldview` | `belief bundle` | Less doctrinal and easier to operationalize. |
| `falsifier` | `counterexample check` | Much clearer as a working mechanism. |
| `demurrage` | `retention pressure` | Avoids leading with an economic metaphor. |
| `BusReceiver` | `Subscription` | Better matches broadcast semantics. |
| `dashboard` | `ops console` or `workspace console` | Better product language for the browser surface. |
| `Daimon` | `AffectBias` | Keeps the meaning, drops the philosophical overhead. |

## Replace or avoid unless there is a hard need

| Term | Recommendation | Why |
|---|---|---|
| `Datum` | avoid canonizing | Too abstract and weakens the two-medium story. |
| `Signal` | avoid reclaiming | Too much historical baggage in this repo. |
| `Event` | avoid as the core noun | Too generic to carry architectural weight. |
| `Message` | avoid as the core noun | Collides with chat semantics and general transport vocabulary. |
| `Envelope` | keep private only | Good as transport scaffolding, bad as the main concept. |
| `marketplace` | avoid near-term | Pushes the docs into ecosystem theater too early. |
| `full parity` | avoid near-term | Encourages fake symmetry instead of shared semantics. |

## Recommended canonical vocabulary

If the redesign is tightened, the public concept stack should read roughly like
this:

- durable record: `Engram`
- live message: `Pulse`
- storage fabric: `Substrate`
- transport fabric: `Bus`
- routing handle: `Topic`
- read model: `Projection`
- projection host: `StateHub`
- user work object: `Session`
- control logic: `Policy`
- learning logic: `Calibrator`
- reusable judgment rule: `HeuristicSpec`
- evidence and performance record: `Calibration`
- user/domain bundle: `domain profile`
- deployment form: `runtime shape`

## Practical consequence

The redesign should reduce, not increase, the number of names a new engineer
must internalize before they can navigate the system. Terms that do not buy a
clear seam should stay private, secondary, or experimental.

--- END 07-naming-and-term-cuts.md ---

--- BEGIN 08-simpler-target-architecture.md ---

# Simpler Target Architecture

This file proposes leaner mechanisms for the areas where the original
refinements got too abstract, too universal, or too hard to validate.

## The simpler shape

The target-state can be described without the heaviest rhetoric:

1. durable records live in `Substrate`;
2. live messages move on `Bus`;
3. user surfaces consume `Projection`s;
4. `Policy` reacts;
5. `Calibrator` learns;
6. domain profiles package tools, gates, and defaults;
7. safety and observability wrap the whole system.

That is already enough architecture to build a strong product.

## Better replacements for the most overbuilt mechanisms

### 1. Replace universal active inference with expectation/outcome loops

Instead of:
- every operator as a full active-inference agent;
- broad `prediction.error.*` ideology everywhere;

prefer:
- operator-local expectation/outcome records;
- calibration updates on observed mismatch;
- bandits or threshold tuning where outcomes are frequent.

Why this is better:
- easier to test;
- easier to explain;
- still gives learning loops without importing a total theory.

### 2. Replace demurrage-first memory with retention tiers plus pressure

Instead of leading with:
- demurrage as the memory model;

prefer:
- hot / warm / cold retention tiers;
- promotion and demotion thresholds;
- optional retention pressure as a tuning factor;
- reinforcement based on use, citation, and surprise.

Why this is better:
- operators already understand tiered retention;
- the system can ship useful memory behavior before nailing the economics;
- the docs stop overselling one metaphor.

### 3. Replace worldview algebra with contradiction management

Instead of:
- worldview objects;
- dissonance stacks;
- broad epistemic theater;

prefer:
- typed claims;
- heuristic specs;
- contradiction queue;
- challenger slots and re-test workflows.

Why this is better:
- contradictions become work items;
- diversity gets a concrete mechanism;
- the system stays empirical rather than philosophical.

### 4. Replace c-factor control doctrine with coordination health plus challengers

Instead of:
- a single collective-intelligence scalar driving runtime behavior;

prefer:
- `coordination health` as an observability concept first;
- challenger slots to keep alternative strategies alive;
- periodic re-tests of minority heuristics;
- explicit cohort-health projections in UI.

Why this is better:
- the signal can mature before it governs decisions;
- users can inspect it without treating it as magic;
- diversity is maintained by policy, not one number.

### 5. Replace registry-first platform language with capability-first lifecycle

Instead of:
- marketplace, registry, ecosystem, ABI promises;

prefer:
- local install;
- inspect and audit;
- explicit enable;
- capability-scoped runtime behavior;
- disable and remove.

Why this is better:
- safer local-first extensibility;
- better operator control;
- less platform theater.

### 6. Replace raw-event UX with projection-first UX

Instead of:
- exposing raw transport concepts to most users;

prefer:
- projection and session streams as the public product model;
- raw topics for privileged and debug consumers only;
- one query+subscribe contract for all surfaces.

Why this is better:
- simpler mental model;
- more stable clients;
- less surface-specific state code.

## Architecture cuts that improve the redesign immediately

### Cut 1. Do not make `Datum` a public doctrine

If a shared either-medium enum is useful internally, keep it internal or narrow.
Do not make it the center of the architecture story.

### Cut 2. Do not standardize all transports and all projections at once

First define:
- cursor;
- subscription;
- state;
- delta;
- query;
- replay.

Everything else can layer on top.

### Cut 3. Do not let one noun carry both user meaning and implementation meaning

Examples:
- `Projection` vs `StateHub`
- `Policy` vs `Calibrator`
- `domain profile` vs `runtime shape`

This cut alone removes a lot of ambiguity.

### Cut 4. Do not let safety become governance sprawl

Keep a small set of enforceable contracts:
- action authorization;
- capability boundaries;
- provenance and audit;
- human approval points.

### Cut 5. Do not let the browser define the product model

The web surface should prove the shared projection contract, not invent parallel
state machinery or demand premature parity.

## Lean build order

If the redesign follows this simpler target, the first meaningful sequence is:

1. three-lane rule and canonical vocabulary;
2. cursor, subscription, projection contracts;
3. session as shared work object;
4. policy/calibrator split;
5. heuristic spec/calibration split;
6. contradiction queue;
7. capability-gated plugin lifecycle;
8. retention tiers, then optional retention pressure;
9. coordination-health projections, then optional actuation.

## Short conclusion

The strongest version of the redesign is not smaller in ambition. It is smaller
in doctrine. It keeps the powerful seams and drops the parts that try to make
the whole system answer to one theory, one metric, or one metaphor.

--- END 08-simpler-target-architecture.md ---

--- BEGIN 06-second-pass-additions.md ---

# Second-Pass Additions

This file adds net-new candidate refinements beyond the original 35. These are
not code-reality complaints. They are missing target-state seams that would
make the redesign more buildable, more legible, and easier to operate.

## High-priority additions

### ADD01. Three-lane kernel rule

The redesign needs one explicit law:
- durable record lane: `Engram` + `Substrate`;
- live transport lane: `Pulse` + `Bus`;
- derived view lane: `Projection` + `StateHub` host.

If a public concept straddles lanes, split it.

Why add this:
- it reduces category confusion;
- it makes docs easier to reason about;
- it gives a hard test for future abstractions.

### ADD02. Cursor and subscription contract

The public model should resume from a `Cursor`, not expose raw bus sequence
numbers as the main mental model.

Also promote `Subscription` as the public lifecycle object:
- subscribe;
- receive state or events;
- resume from cursor;
- unsubscribe.

Why add this:
- replay and resume become one consistent story;
- CLI, TUI, web, and external clients can share semantics;
- it prevents low-level transport details from leaking upward.

### ADD03. Projection versioning and migration

Every projection should carry:
- schema version;
- compatibility policy;
- migration rule;
- tests for state and delta evolution.

Why add this:
- projections will become a public contract;
- UI and integrations will otherwise be brittle;
- versioning discipline is cheaper than projection churn.

### ADD04. Session as the shared work object

Make `session` the cross-surface unit of work. A session should own:
- transcript;
- cursor;
- permissions;
- active domain profile;
- replay state;
- current task or plan lineage.

Why add this:
- "resume" becomes portable across CLI, TUI, chat, and web;
- users think in sessions more naturally than in transport internals;
- the product gains one stable object to organize around.

### ADD05. Command registry behind every surface

Define commands once, render them many ways:
- CLI subcommand;
- slash command;
- command palette;
- button or menu item.

Why add this:
- surface parity becomes a rendering problem instead of a documentation problem;
- help text, permissions, and auditability can attach to one command registry;
- UX stops drifting across interfaces.

### ADD06. Split Policy from Calibrator

`Policy` should react and decide.
`Calibrator` should learn from outcomes and update weights, thresholds, and
heuristics.

Why add this:
- control and learning are different jobs;
- it prevents one omnipotent "policy" bucket from swallowing everything;
- it creates a cleaner seam for experiments.

### ADD07. Split HeuristicSpec from Calibration

A heuristic should not be one undifferentiated blob. Split it into:
- `HeuristicSpec`: rule, scope, rationale, counterexample shape;
- `Calibration`: hit rate, drift, confidence, challenge history.

Why add this:
- rules and evidence evolve at different speeds;
- the UI can show "what this heuristic says" separately from "how well it is
  doing";
- promotion and retirement become simpler.

### ADD08. Contradiction queue

When two applicable heuristics disagree, create a first-class work item instead
of treating disagreement as a vague runtime mood.

A contradiction queue should track:
- conflicting heuristics or claims;
- affected domain or task;
- severity;
- owner;
- resolution state.

Why add this:
- contradiction becomes actionable;
- it gives learning a concrete backlog;
- it is a smaller, better mechanism than broad worldview algebra.

### ADD09. Citation gate for research-derived knowledge

Before external research becomes a runtime claim or heuristic, require:
- resolvable source;
- quote or evidence extraction;
- citation validity check;
- provenance record.

Why add this:
- it blocks folklore from entering the system as truth;
- it makes research-to-runtime legible;
- it is much cheaper than a full replication ledger first.

### ADD10. Intent fingerprint for risky actions

For risky domains, represent intended action in a typed form and require
proposed execution to match it before approval or dispatch.

Applies especially to:
- blockchain transactions;
- production ops changes;
- destructive file or deployment actions.

Why add this:
- it is a concrete safety mechanism;
- it is more buildable than abstract custody rhetoric alone;
- it creates a clean preflight gate.

### ADD11. Capability-gated plugin lifecycle

Do not make plugin discovery equivalent to plugin activation.

Recommended lifecycle:
1. install
2. inspect
3. audit
4. enable
5. observe
6. disable or remove

Why add this:
- local-first extensibility gets a safety story without registry theater;
- permissions become explicit;
- operators can reason about extension state.

### ADD12. State movement verbs

`export/import` is too coarse. Add explicit operations for runtime state:
- backup;
- restore;
- clone;
- migrate;
- split;
- merge.

Why add this:
- deployment docs become more operationally useful;
- state movement can be dry-run and validated;
- multi-instance and team workflows get a cleaner story.

### ADD13. Maturity bands for every major concept

Mark major concepts as one of:
- `current`
- `target`
- `experimental`
- `research`

Why add this:
- the docs stop flattening all ideas into one confidence level;
- readers can separate redesign core from speculative extensions;
- roadmap arguments become easier to police.

## Best new near-term sequence

If these additions are adopted, the best early order is:

1. three-lane rule;
2. cursor and subscription contract;
3. projection versioning;
4. session as shared work object;
5. policy/calibrator split;
6. heuristic spec/calibration split;
7. contradiction queue;
8. capability-gated plugin lifecycle;
9. citation gate and intent fingerprint;
10. state movement verbs and maturity bands.

## Short conclusion

The original refinement set improved ambition. These additions improve
buildability. They make the target-state less mystical and more like a system
with clear contracts, public objects, and reversible operations.

--- END 06-second-pass-additions.md ---

## Master Summary (reference)

# Refinements Audit — Master Summary

> **Date**: 2026-04-17 | **Auditor**: Claude Opus 4.6 (7 parallel agents)
> **Scope**: All 35 refinement docs + runner infrastructure + landed doc updates + codebase reality check
> **Output**: 7 detailed audits in this directory (01-foundation through 07-doc-quality)

---

## Executive Verdict

**The diagnosis is correct. The prescription is 5-10x overscoped.**

The refinements correctly identify real problems in the codebase (event enum proliferation, a conductor/learn layer violation, stale "Signal" naming, Policy signature mismatch). But they propose a 6-12 month, 5-7 engineer refactoring program for a single-developer project, introducing ~15 types that don't exist yet (Pulse, Datum, Bus trait, TopicFilter, Demurrage, Custody, Worldview, Claim, Paper, TypedContext, etc.) to solve problems that could be fixed in ~1-2 weeks with targeted changes.

---

## The 5 Things to Ship Now

These emerged consistently across all 7 audit workstreams as high-value, low-risk:

| # | What | Where | Effort | Why |
|---|---|---|---|---|
| 1 | **Add HDC fingerprint field to Engram** | `roko-core/src/engram.rs` | 1 day | HdcVector exists (10,240-bit, tested). Episode fingerprinting already works. This is the single highest-value bridge between the learning and memory layers. |
| 2 | **Unify event enums into `RokoEvent`** | Across 4 crates | 1 week | Four incompatible event enums (2x `AgentEvent`, `RokoEvent`, `ServerEvent`) is the real problem. Unify them. |
| 3 | **Add generic `Bus<E>` trait to roko-core** | `roko-core/src/traits.rs` | 2-3 days | ~100 lines. Keep it generic (not Pulse-specific). Solves the layer violation. |
| 4 | **Clean up stale "Signal" references** | traits.rs, README, kind.rs, CLAUDE.md | 1 hour | 40+ stale occurrences across docs and code comments. |
| 5 | **Fix architecture INDEX status** | `docs/00-architecture/INDEX.md` | 30 min | Says "roko-serve: HTTP API not wired" and "TUI: Text-mode dashboard only" — both factually wrong per CLAUDE.md and code (30K LOC serve, 58K LOC TUI). |

---

## The 5 Things to Ship Soon (next month)

| # | What | Source | Effort |
|---|---|---|---|
| 6 | **CLI parity / muscle memory (REF28)** | UX audit | 1-2 weeks |
| 7 | **StateHub hardening (REF26)** | UX audit | 1 week |
| 8 | **Heuristic calibration struct** | Learning audit (REF14) | 3-5 days |
| 9 | **Safety: extend Attestation + expand taint** | Integrator audit (REF32) | 1 week |
| 10 | **Threat model doc** | Integrator audit (REF32 §13) | 2 days |

---

## The 10 Things to Defer

| What | Why defer |
|---|---|
| **Pulse type** (REF02) | Unified `RokoEvent` enum solves the same problem more simply |
| **Datum enum** (REF04) | Premature abstraction; doubles every trait's surface area |
| **Operator generalization** (REF04) | Only Policy actually needs a signature change |
| **Demurrage** (REF12) | Add `last_used + access_count` to Decay first; skip the full economic model |
| **Plugin SPI tiers 4-5** (REF17) | Zero plugin authors exist. WASM host is premature |
| **3 new kernel crates** (REF20) | roko-bus justified, roko-hdc unnecessary (345 LOC), roko-spi premature |
| **All 5 rewrite candidates** (REF21) | Existing code works. Build incrementally |
| **SvelteKit web UI** (REF29) | Zero frontend code exists. Build when someone asks |
| **gRPC wire protocol** (REF27) | No tonic dependency. WebSocket + SSE already work |
| **12-month roadmap timeline** (REF35) | Calibrated for 5-7 engineers, not 1 developer + AI |

---

## The 5 Things That Are Wrong

| What | Issue | Source |
|---|---|---|
| **Synergy matrix** (REF31) | 7 of 10 "load-bearing primitives" don't exist in code. Matrix is aspirational fiction. | Integrator audit |
| **REF32 ignores existing safety system** | The AgentContract/AgentWarrant/Capability system already exists and works. REF32 proposes replacing it without acknowledging it. | Integrator audit |
| **Glossary marks EventBus as "retired"** | `EventBus<E>` is the only live transport code. No Bus trait or Pulse exists. | Integrator audit |
| **"Moat" framing** (REF18) | Of 10 claimed moat components, 2 exist fully, 2 partially, 6 not at all. The moat is aspirational. | Moat audit |
| **Doc INDEX says serve/TUI "not wired"** | serve has 200+ routes (30K LOC), TUI has 58K LOC with WebSocket. Both are definitively wired. | Doc quality + reality check |

---

## Codebase Reality (Key Numbers)

From the reality-check audit:

| What | Reality |
|---|---|
| Total Rust LOC | 322,088 (not 177K as CLAUDE.md says) |
| Workspace members | 36 (not 18) |
| Test functions | 3,761 |
| orchestrate.rs | 17,087 lines (the integration hairball) |
| roko-serve routes | 200+ (not ~85) |
| TUI code | 58K LOC |
| roko-learn modules | 42 modules, 35,847 LOC |
| Signal→Engram rename | 99.6% complete (4 real stragglers) |
| Event bus event types | Exactly 2 (PlanRevision, PrdPublished) |
| Demurrage in code | 0 lines |
| Pulse in code | 0 lines |
| Worldview in code | 0 lines |

---

## Doc Quality Assessment

Overall: **3.8 / 5**

**Good**: No copy-paste artifacts. Glossary is excellent. Synergy map and safety spine read as unified docs. Cross-references resolve.

**Issues**:
1. "Signal" still used in ~40 places across 8+ pre-existing docs
2. Target crates (roko-bus, roko-hdc, roko-spi) described in present tense as if they exist
3. Architecture INDEX has stale status information contradicting CLAUDE.md

---

## Per-Arc Summary

### Foundation (01-09): PARTIALLY AGREE
The diagnosis is correct. The prescription (Pulse, Datum, generalized operators, 7-step TickConfig) is overcomplicated. Fix: unify events, add generic Bus trait, update docs. ~1 week instead of 6-7 weeks.

### Learning (10-16): SIMPLIFY
The docs undercount what already exists. roko-learn has 42 modules and 36K LOC. HDC fingerprint field on Engram is the highest-value change. Demurrage/worldviews/replication-ledger are premature.

### Moat (17-21): DEFER/SKEPTICAL
Zero plugin authors, zero external users. The moat is aspirational. Plugin tier 3 (tool manifests) is useful later. Everything else waits.

### UX (22-30): Pick 3 of 9
Ship REF28 (CLI parity), REF26 (StateHub), and the chat/init subset of REF23. Defer the four-layer SDK, six domain profiles, SvelteKit UI, gRPC, and rich UX primitives.

### Integrators (31-35): Integrate code, not plans
The synergy matrix, glossary, and roadmap are plans connecting to plans. Ship: threat model, glossary (split into "exists" vs "planned"), dependency ordering. Reject: quarterly timeline, synergy matrix of unbuilt features.

---

## Recommended Priority Queue

For a single developer + AI agents:

1. **Close the self-hosting loop** (CLAUDE.md items 10-11: auto plan generation + feedback loop)
2. Ship the 5 "now" items above
3. Ship the 5 "soon" items above
4. Address ux-followup P0 items (67 items in `tmp/ux-followup/`)
5. Decompose `orchestrate.rs` (17K lines is the real tech debt)
6. Everything else goes into "when the system needs it"

---

## Audit Files

| File | What |
|---|---|
| `01-foundation-audit.md` | REF01-09 vs codebase (28K chars) |
| `02-learning-audit.md` | REF10-16 vs codebase (30K chars) |
| `03-moat-audit.md` | REF17-21 vs codebase (25K chars) |
| `04-ux-audit.md` | REF22-30 vs codebase (25K chars) |
| `05-integrator-audit.md` | REF31-35 vs codebase (23K chars) |
| `06-codebase-reality-check.md` | 10 factual claims verified (27K chars) |
| `07-doc-quality-audit.md` | Landed doc updates quality (18K chars) |

## Refinement Matrix (per-REF verdicts)

# Refinement Matrix

Legend:
- `keep`
- `narrow`
- `defer`
- `rewrite`

| Ref | Title | Verdict | Audit note |
|---|---|---|---|
| REF01 | critique one noun | `keep` | The diagnosis is real: transport is under-modeled and the kernel story is too storage-centric. |
| REF02 | Engram vs Pulse | `keep` | `Pulse` is a good transport noun if used to clarify the redesign rather than force a total renaming campaign. |
| REF03 | Bus as first class | `keep` | This is the strongest foundational follow-up: unify and formalize transport. |
| REF04 | operators generalized | `narrow` | Good local idea, bad universal law. Medium polymorphism should be proven operator by operator. |
| REF05 | loop retold | `keep` | Useful as a reference architecture for the redesign, but should guide migration rather than dictate every interface immediately. |
| REF06 | refactoring plan | `keep` | A phased migration plan is appropriate; keep it honest and code-first. |
| REF07 | naming | `narrow` | Good cleanup instinct, but not every proposed term should become top-level canon immediately. |
| REF08 | code sketches | `narrow` | Helpful as exploratory sketches; should not be confused with settled API design. |
| REF09 | phase-2 implications | `narrow` | Good future map, but it should stay downstream of core runtime wins instead of shaping the first redesign pass. |
| REF10 | self-learning loops | `keep` | Strong direction if centered on calibration, contradiction, and adaptation rather than runtime-wide active-inference doctrine. |
| REF11 | HDC substrate | `narrow` | Keep HDC for retrieval/clustering; defer broader semantic-consensus rhetoric. |
| REF12 | knowledge demurrage | `defer` | Interesting hypothesis, but too early to present as the governing memory model. |
| REF13 | c-factor | `defer` | Worth exploring as coordination health, not yet worthy of strong canonical treatment. |
| REF14 | worldview validation | `narrow` | Keep typed heuristics and contradiction tracking; defer full worldview/dissonance stack. |
| REF15 | exponential scaling | `defer` | Too much product-theory confidence for the current maturity level. |
| REF16 | research-to-runtime | `narrow` | Claim registry and provenance-backed defaults are promising; the full paper economy is premature. |
| REF17 | plugin extension architecture | `keep` | Tiered extensibility is the right platform direction if it stays local-first and resists premature ecosystem ambition. |
| REF18 | competitive moat | `defer` | Too much architecture-theater and future-ecosystem assumption. |
| REF19 | net-new innovations | `rewrite` | The catalog format oversells speculative pieces; convert to research hypotheses or remove. |
| REF20 | modularity composability | `keep` | Crate-boundary cleanup and clearer seams are real needs. |
| REF21 | from-scratch redesigns | `narrow` | Useful as a pressure test and cleanup lens, but dangerous as the default implementation mindset. |
| REF22 | developer UX rust | `keep` | Strong redesign target if the SDK is kept crisp and optimized for time-to-first-agent rather than feature taxonomy. |
| REF23 | user UX running agents | `keep` | Strong target-state direction if parity follows a real shared session model instead of surface symmetry for its own sake. |
| REF24 | deployment UX | `keep` | Strong operator-centered direction; needs stricter sequencing and fewer assumptions bundled into the first wave. |
| REF25 | domain-specific agents | `keep` | Domain profiles are a strong packaging abstraction as long as bundles stay ahead of universal type formalism. |
| REF26 | StateHub rearchitecture | `keep` | One of the best proposals. Evolve the existing dashboard hub into real projections. |
| REF27 | realtime event surface | `keep` | Unification is the right target, but the contract should stay small: events, replay, filters, subscriptions. |
| REF28 | CLI parity familiar workflows | `keep` | Familiar-first is right if parity is earned from shared workflow semantics rather than copied command names. |
| REF29 | web UI architecture | `keep` | A web surface is a good redesign goal if it starts as an ops console and grows from projection contracts. |
| REF30 | rich UX primitives | `narrow` | Some primitives are valuable, but only when supported by real shared state and telemetry contracts. |
| REF31 | synergy integration map | `defer` | Fine as internal coherence tooling; too grand as canonical architecture backmatter. |
| REF32 | safety sandbox provenance | `keep` | Strong direction if safety remains a compact enforceable spine rather than an all-at-once governance superstructure. |
| REF33 | observability telemetry | `keep` | Strong direction if the signal set stays operator-useful and avoids speculative overmodeling. |
| REF34 | glossary | `rewrite` | Keep one glossary, but split current canon from target-state proposals. |
| REF35 | consolidated roadmap | `rewrite` | Keep sequencing discipline, but narrow the number of simultaneous deep bets and remove unearned quarter-level certainty. |

## Aggregated view

### Clear keeps

- REF01
- REF02
- REF03
- REF05
- REF06
- REF10
- REF17
- REF20
- REF22
- REF23
- REF24
- REF25
- REF26
- REF27
- REF28
- REF29
- REF32
- REF33

### Strong, but should be narrowed

- REF04
- REF07
- REF08
- REF09
- REF11
- REF14
- REF16
- REF21
- REF30

### Better deferred

- REF12
- REF13
- REF15
- REF18
- REF31

### Need substantive rewrite

- REF19
- REF34
- REF35

## Practical consequence

The refinement set should not be treated as a monolithic "land it all" bundle.
The right next pass is:

1. Preserve the `keep` items.
2. Rewrite the `narrow` items around smaller scope and less doctrinal force.
3. Move the `defer` items into explicit future-work or research-hypothesis sections.
4. Rebuild the `rewrite` items so they stop acting as authority multipliers for
   architecture that is still too speculative or too overloaded.

# Batch AUD08: Apply naming/term cuts and simpler target architecture

**Audit refs**: 07-naming-and-term-cuts.md (full file), 08-simpler-target-architecture.md
(full file), 01-executive-summary.md (recommended next moves). This is the
final Phase 1 batch -- it applies the audit's recommended vocabulary tightening
and architecture simplification across the docs.

Read these files first:

- `tmp/refinement-audit-runner/context-pack/00-AUDIT-RULES.md`
- `tmp/refinements-audit/07-naming-and-term-cuts.md` (full file -- naming recommendations)
- `tmp/refinements-audit/08-simpler-target-architecture.md` (full file -- simpler mechanisms)
- `tmp/refinements-audit/01-executive-summary.md` (recommended next moves)
- `tmp/refinements-audit/00-MASTER-SUMMARY.md` ("The 5 Things That Are Wrong" section)
- `tmp/refinements-audit/02-foundation-learning.md` (glossary hardening section)
- `docs/00-architecture/01-naming-and-glossary.md` (the canonical glossary -- this is the primary edit target)
- `docs/00-architecture/INDEX.md` (the architecture lead-in)
- `docs/INDEX.md` (top-level, "Current Framing" block)
- `docs/00-architecture/00-vision-and-thesis.md`
- `docs/00-architecture/11-dual-process-and-active-inference.md`
- `docs/00-architecture/14-c-factor-collective-intelligence.md`
- `docs/00-architecture/04-decay-variants.md`

## Task

Apply the audit's naming and terminology recommendations to make the docs
clearer for a new reader. The audit identified terms that are overloaded,
overly doctrinal, or confusing. It also identified simpler mechanisms that
should replace the current heavy abstractions in the target architecture
descriptions. This batch applies those recommendations.

## Current state (evidence)

### Naming issues found by the audit

1. **`TypedContext`** -- opaque jargon. Better: **`Situation`** ("more intuitive
   for users and operators").

2. **`c-factor`** -- requires prior literature knowledge. Better: **`coordination
   health`** in public-facing docs (keep `c-factor` as the internal metric name).

3. **`worldview`** -- doctrinal. Better: **`belief bundle`** in public-facing
   docs.

4. **`falsifier`** -- opaque. Better: **`counterexample check`** in public-facing
   docs.

5. **`demurrage`** -- leads with economic metaphor. Better: **`retention
   pressure`** in public-facing docs.

6. **`Daimon`** -- philosophical overhead. Consider adding "AffectBias" as the
   public-facing alias.

7. **`Datum`** -- too abstract, weakens the two-medium story. Avoid canonizing
   in public docs.

8. **`dashboard`** -- better: **`ops console`** or **`workspace console`** for
   the browser surface.

9. **Profile overloading** -- "profile" is used for both domain profiles and
   deployment shapes. Split: **`domain profile`** (tools/roles/defaults) vs.
   **`runtime shape`** (laptop/server/container/cluster).

10. **`Policy` overloading** -- does both control and learning. The audit
    suggests splitting into `Policy` (control) + `Calibrator` (learning) in
    docs.

### Architecture simplification recommendations

The audit recommends replacing heavy doctrine with simpler mechanisms:

| Heavy doctrine | Simpler mechanism |
|---|---|
| Universal active inference for all operators | Expectation/outcome loops per operator |
| Demurrage-first memory model | Retention tiers + optional pressure |
| Worldview algebra | Contradiction management (typed claims + challenger slots) |
| c-factor control doctrine | Coordination health as observability, then optional actuation |
| Registry-first platform | Capability-gated local plugin lifecycle |
| Raw-event UX | Projection-first UX |

### The "Current Framing" wall-of-text

`docs/INDEX.md` lines 171-214 form a single growing paragraph that was
appended to by successive refinements. Each REF added another clause. The
result is a 150-line block that no developer will read. The doc quality
audit calls this a P1 issue.

## Implementation

### 1. Add public-facing aliases to the glossary

In `docs/00-architecture/01-naming-and-glossary.md`:
- For each term in the naming-cuts table, add a "Public alias" or
  "User-facing name" note in the glossary entry. Do NOT rename the internal
  terms -- just add the clearer alias for docs/UI/CLI contexts.
  Examples:
  - `TypedContext` entry: add "Public alias: **Situation**"
  - `c-factor` entry: add "Public alias: **coordination health**"
  - `worldview` entry: add "Public alias: **belief bundle**"
  - `falsifier` entry: add "Public alias: **counterexample check**"
  - `demurrage` entry: add "Public alias: **retention pressure**"
- Add a new entry for `Calibrator` as the proposed learning-logic split from
  Policy, marked `[planned]`
- Add a new entry for `runtime shape` to distinguish from `domain profile`
- Add a note in the glossary introduction explaining the public-alias
  convention: "Some terms have a public alias used in user-facing docs, CLI
  output, and UI. The internal term remains canonical in code and architecture
  docs."

### 2. Apply simpler framing in architecture narrative docs

In `docs/00-architecture/11-dual-process-and-active-inference.md`:
- Where "every operator is a predictor" doctrine appears, soften to:
  "Operators that make discrete, measurable choices (especially Router) benefit
  most from prediction/outcome loops. Universal operator prediction is a
  target-state aspiration, not a first-pass requirement."
- Where FEP/Friston is cited as the governing theory, add: "The engineering
  mechanism is simpler: expectation/outcome records per operator, with
  calibration updates on mismatch."

In `docs/00-architecture/14-c-factor-collective-intelligence.md`:
- Where c-factor is presented as a control input, reframe:
  "Near-term: c-factor (coordination health) is an observability metric.
  Target-state: once the signal matures, it can optionally drive Policy
  interventions."

In `docs/00-architecture/04-decay-variants.md`:
- Where demurrage is the lead framing, add the simpler alternative:
  "The simpler near-term mechanism is retention tiers (hot/warm/cold) with
  promotion/demotion thresholds and optional retention pressure. Full
  demurrage economics is a target-state extension."

### 3. Collapse the INDEX.md "Current Framing" wall-of-text

In `docs/INDEX.md`:
- Rewrite the "Current Framing" block (lines ~171-214) from a single accretive
  paragraph into a structured format:

```markdown
## Current Framing

> The architecture is organized around:
>
> | Concept | What | Key docs |
> |---|---|---|
> | **Two mediums** | Engram (durable) + Pulse (ephemeral, planned) | [02-engram](00-architecture/02-engram-data-type.md), [02b-pulse](00-architecture/02b-pulse-ephemeral-event.md) |
> | **Two fabrics** | Substrate (storage) + Bus (transport, planned) | [07-substrate](00-architecture/07-substrate-trait.md), [07b-bus](00-architecture/07b-bus-transport-fabric.md) |
> | **Six operators** | Scorer, Gate, Router, Composer, Policy, Substrate | [06-traits](00-architecture/06-synapse-traits.md) |
> | **Learning** | Prediction/outcome loops, bandits, skill library | [05-learning/INDEX](05-learning/INDEX.md) |
> | **HDC** | 10,240-bit fingerprints for similarity | [02-engram](00-architecture/02-engram-data-type.md) |
> | **Safety** | 5-policy chain, contracts, warrants | [11-safety/INDEX](11-safety/INDEX.md) |
>
> For canonical vocabulary, see [Naming and Glossary](00-architecture/01-naming-and-glossary.md).
```

- Remove the per-REF citation sentences. The individual docs already have
  proper cross-references.

### 4. Clean up architecture INDEX lead-in

In `docs/00-architecture/INDEX.md`:
- The opening paragraph (lines 1-28) is also accretive with many REF citations.
  Tighten it to focus on the core story without per-REF citations.
- Keep the "two mediums, two fabrics, six operators" framing but remove the
  inline `tmp/refinements/` references. Those belong in source-tracking
  metadata, not the reader's introduction.

### 5. Apply Profile -> domain profile / runtime shape split in docs

Search the docs tree for places where "profile" is used ambiguously:
- Where it means tools/roles/defaults: clarify as "domain profile"
- Where it means laptop/server/container/cluster: clarify as "runtime shape"
- Focus on `docs/19-deployment/INDEX.md` and `docs/12-interfaces/` where the
  ambiguity is most confusing

## Write scope

Primary:
- `docs/00-architecture/01-naming-and-glossary.md` (public aliases + new entries)
- `docs/INDEX.md` (collapse Current Framing wall-of-text)
- `docs/00-architecture/INDEX.md` (tighten lead-in)

Secondary:
- `docs/00-architecture/11-dual-process-and-active-inference.md`
- `docs/00-architecture/14-c-factor-collective-intelligence.md`
- `docs/00-architecture/04-decay-variants.md`
- `docs/00-architecture/00-vision-and-thesis.md` (only if it has heavy doctrine)

Tertiary (profile split):
- `docs/19-deployment/INDEX.md`
- `docs/12-interfaces/` files that use "profile" ambiguously

## Rules

1. **Add aliases, do not rename.** Internal terms (`TypedContext`, `c-factor`,
   `demurrage`) stay in the code and architecture docs. Public aliases are
   added for user-facing contexts.
2. **Soften doctrine, do not remove it.** The active-inference, demurrage, and
   c-factor ideas are valuable research directions. Reframe them as
   target-state aspirations with simpler near-term mechanisms.
3. **The INDEX rewrite is the highest-impact change.** The wall-of-text is the
   P1 doc-quality issue. A clean table or list makes the docs navigable.
4. **Do not touch files already fully handled by AUD01-AUD07** unless applying
   a naming/framing fix that those batches did not address.
5. **Do not rename types in code snippets.** If a code snippet uses
   `TypedContext`, leave it. Add a prose note that the public alias is
   `Situation`.
6. **Keep `tmp/refinements/` references in source-tracking sections** (like
   "Generation Notes" at doc bottoms) but remove them from reader-facing
   introductions and overviews.
7. **Do not add new sections to docs.** This batch tightens existing content;
   it does not add new design material.

## Done when

- Glossary has public aliases for TypedContext, c-factor, worldview, falsifier,
  demurrage, and Daimon
- Glossary has entries for Calibrator and runtime shape
- Glossary introduction explains the public-alias convention
- `docs/INDEX.md` "Current Framing" is a structured table/list, not a
  wall-of-text
- `docs/00-architecture/INDEX.md` lead-in is clean of per-REF inline citations
- Active inference, c-factor, and demurrage docs have simpler near-term
  mechanisms presented alongside the target-state doctrine
- "Profile" ambiguity is resolved in deployment and interface docs
- No internal type names were renamed
- Final message lists: (a) public aliases added to glossary, (b) the old and
  new shape of the INDEX.md Current Framing block, (c) number of files where
  doctrine was softened, (d) number of files where profile was disambiguated
