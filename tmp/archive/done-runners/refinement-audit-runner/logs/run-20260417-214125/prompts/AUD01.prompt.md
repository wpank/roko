# Refinement Audit Runner — Batch AUD01

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

--- BEGIN 00-MASTER-SUMMARY.md ---

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

--- END 00-MASTER-SUMMARY.md ---

--- BEGIN 01-executive-summary.md ---

# Executive Summary

## Bottom line

The refinement set has a strong architectural core and is worth using as a
design reference for what should be built next.

Its best parts point toward the right redesign:
- make transport first-class instead of implicit;
- make learning loop through explicit calibration and contradiction handling;
- make StateHub/projections the bridge from runtime to interfaces;
- make extension surfaces modular, local-first, and composable;
- make safety, provenance, and observability explicit architectural contracts.

Its weakest parts are not "too futuristic." They are too totalizing. Several
proposals try to become universal law before they have earned that status.
That would make the redesign harder, more brittle, and more doctrinal than it
needs to be.

## The main verdict

The refinements should not be rolled back. They should be tightened into a
stricter redesign blueprint:

1. Keep the diagnosis.
2. Keep the target-state ambition.
3. Narrow the number of concepts that become universal architecture.
4. Prefer strong seams and contracts over system-wide metaphors.
5. Sequence around a few compounding runtime wins instead of a grand rewrite.

## What is directionally right

### 1. Storage vs transport is a real missing axis

Treating transport as a first-class architectural concern is the right move.
That gives the redesign a cleaner runtime story than a storage-only kernel.

The strongest version of this idea is:
- formalize a shared transport contract;
- let projections, observers, and learning consume the same stream;
- use `Bus` as a real runtime seam;
- avoid making every downstream type system bend around one universal transport
  doctrine.

### 2. Evented calibration is the right learning pattern

The redesign is strongest when learning means:
- prediction or expectation;
- observed outcome;
- contradiction, drift, or surprise;
- updated heuristic or routing choice.

That is concrete, extensible, and inspectable. It is much stronger than making
"active inference" the master explanation for the whole runtime.

### 3. A heuristic layer between episodes and playbooks is valuable

This is one of the strongest ideas in the entire set. It gives Roko a middle
layer between raw run history and distilled playbooks. That is likely to create
real leverage:
- reusable judgment without overfitting to one run;
- inspectable defaults and fallbacks;
- a place to store contradiction and calibration history.

### 4. StateHub as shared projection infrastructure is a strong direction

This is one of the cleanest target-state ideas. A projection layer is the right
way to connect runtime events to CLI, TUI, chat, and web without forcing each
surface to invent its own query and replay semantics.

### 5. Low-power extension tiers are the right platform story

Tier 1/2/3 style extension surfaces are promising:
- prompt packs,
- profile bundles,
- manifest tools,
- MCP adapters.

The platform story is best when it stays local-first, inspectable, and easy to
reason about. It gets worse when it jumps immediately to ecosystem theater.

## What is wrong or overbuilt

### 1. Too many proposals try to become universal law

- all operators as dual-medium polymorphs;
- demurrage as the governing memory economy;
- c-factor as an operational intelligence signal;
- worldview/falsifier/dissonance as structured first-class runtime objects;
- research claims as direct runtime config inputs;
- synergy matrices and moat language as if integration itself proves leverage.

Many of these ideas may be worth exploring, but only some are mature enough to
become redesign primitives.

### 2. New jargon expands faster than decision quality

Some new terms are useful. Too many at once creates conceptual drag.

Most defensible:
- `Pulse`
- `Bus`
- `StateHub`
- `TypedContext`
- `Custody`

Most likely too early or too broad as top-level canon:
- `Datum` as universal operator input
- universal active-inference framing
- worldview/dissonance stack
- demurrage economy as primary memory explanation
- synergy/matrix framing as central architectural lens

### 3. Some proposals are stronger as local mechanisms than as global ideology

Examples:
- `Bus` is strong as transport infrastructure, not as the answer to every
  runtime concern.
- heuristics are strong as a middle layer, not as proof of a full worldview
  algebra.
- domain profiles are strong as bundles and constraints, not yet as total typed
  domain kernels.
- observability is strong as emitted signals and replay, not as a giant unified
  meta-model from day one.

## The highest-value reframing

### Keep as near-term architecture

- Formalize transport as a first-class runtime concern.
- Build one event surface that projections and learning can both consume.
- Treat StateHub as the bridge from transport to interface.
- Keep calibration as the core learning pattern.
- Grow a typed heuristic layer carefully.
- Keep plugins/profile bundles local-first and low-power first.
- Make safety and observability explicit contracts.

### Keep as target-state, but with weaker doctrinal force

- full dual-medium operator algebra;
- seven-step loop as a reference architecture;
- generalized active inference across the runtime;
- demurrage as one future memory-economics option;
- c-factor as an experiment rather than a foundational scalar;
- paper/claim/replication-ledger runtime;
- one typed realtime surface across all UX surfaces;
- five-tier plugin ecosystem with registry and ABI guarantees.

### Move to research backlog or hypothesis framing

- superlinear scaling claims;
- moat claims driven by interaction density;
- prediction-market style or consensus-semantics claims around HDC;
- broad worldview algebra;
- externally meaningful commons/network-effect claims before a real external
  usage loop exists.

## Recommended next moves

1. Rewrite the docs so they read as an intentional redesign blueprint, not an
   everything-at-once manifesto.
2. Reduce the number of terms that become repo-wide canon.
3. Make transport unification the first real architectural follow-up.
4. Define a small projection contract and let StateHub grow from that.
5. Build typed heuristics and contradiction tracking before worldview rhetoric.
6. Keep memory economics experimental until simpler learning loops are working.
7. Build Tier 1/2/3 extension capability before any registry or ABI story.
8. Rework the roadmap around compounding runtime milestones with sharp exit
   criteria.

## The single sentence summary

The refinements are strongest when they drive Roko toward cleaner seams, better
learning loops, better projections, and safer extensibility, and weakest when
they try to lock the redesign into an overgeneralized cybernetic ideology.

--- END 01-executive-summary.md ---

--- BEGIN 05-refinement-matrix.md ---

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

--- END 05-refinement-matrix.md ---

# Batch AUD01: Fix stale status, LOC counts, and crate counts in top-level docs

**Audit refs**: 00-MASTER-SUMMARY.md (item 5), 07-doc-quality-audit.md (Issue C),
06-codebase-reality-check.md (sections 1-6). This is the foundation batch --
every subsequent batch assumes these numbers and statuses are correct.

Read these files first:

- `tmp/refinement-audit-runner/context-pack/00-AUDIT-RULES.md`
- `tmp/refinements-audit/00-MASTER-SUMMARY.md`
- `tmp/refinements-audit/06-codebase-reality-check.md` (sections 1-6: reality numbers)
- `tmp/refinements-audit/07-doc-quality-audit.md` (Issue C: stale implementation status)
- `docs/INDEX.md`
- `docs/STATUS.md`
- `docs/00-architecture/INDEX.md` (full file, especially the "Current Status and Implementation Gaps" section near the bottom)
- `CLAUDE.md` (the project CLAUDE.md at repo root, for ground-truth status)

## Task

Update the three top-level navigation docs (`docs/INDEX.md`, `docs/STATUS.md`,
`docs/00-architecture/INDEX.md`) so they reflect the actual codebase state as of
2026-04-17. The refinements-runner wrote these docs but never reconciled the
status sections against reality. The audit found specific factual errors that
must be corrected.

## Current state (evidence)

The audit found these specific errors:

1. **`docs/00-architecture/INDEX.md`** says `roko-serve: HTTP API not wired` --
   WRONG. roko-serve has 200+ routes (30K LOC) and is fully wired. CLAUDE.md
   marks it as **Wired**.

2. **`docs/00-architecture/INDEX.md`** says `TUI: Text-mode dashboard only, no
   interactive terminal UI` -- WRONG. The TUI has 58K LOC of ratatui code with
   F1-F7 tabs, WebSocket integration, and is fully wired. CLAUDE.md marks it
   as **Wired**.

3. **`docs/STATUS.md`** says `Interfaces` section is `Scaffold` with
   `roko-cli (text dashboard)` -- WRONG. The TUI is a full ratatui interactive
   terminal UI, and roko-serve provides the HTTP API.

4. **`docs/STATUS.md`** says `HTTP server + REST API: Crate exists, no routes`
   under Scaffold -- WRONG. roko-serve has 200+ routes.

5. **`docs/STATUS.md`** says `Text dashboard (TUI): Renders text pages, no
   interactive terminal UI` under Scaffold -- WRONG. Full ratatui.

6. **LOC counts**: CLAUDE.md says ~177K LOC and 18 crates. The reality check
   found 322,088 LOC and 36 workspace members. Docs that cite these numbers
   need correction.

7. **Test count**: STATUS.md says `Total: 1,568 tests`. The reality check found
   3,761 test functions. The per-crate breakdown may also be stale.

8. **Route count**: CLAUDE.md says ~85 routes. The reality check found 200+.
   Docs that cite the route count should say 200+.

9. **roko-learn**: STATUS.md says 101 tests. The reality check found 42 modules
   and 35,847 LOC -- the test count may be understated.

10. **Critical Path section in STATUS.md** says `Interactive TUI (Section 12) --
    Wire ratatui into the text dashboard scaffold` -- this is DONE.

## Implementation

### 1. Fix `docs/00-architecture/INDEX.md` status section

Find the "Current Status and Implementation Gaps" section (near the bottom of
the file). Update:

- Change `roko-serve` status from "HTTP API not wired" to "**Shipping**: 200+
  REST routes, SSE, WebSocket on :6677"
- Change TUI status from "Text-mode dashboard only" to "**Shipping**: ratatui
  interactive TUI with F1-F7 tabs, WebSocket, themes, modals"
- Update any stale test counts or LOC numbers in that section
- Update the "Sub-docs" count if it says 29 when there are now 36 files
- Update the generated date if present

### 2. Fix `docs/STATUS.md` master status matrix

- Change section 12 (Interfaces) from `Scaffold` to `Shipping` (at least for
  TUI and HTTP API; some subsections like web portal remain Specified)
- Move `HTTP server + REST API` and `Text dashboard (TUI)` from the Scaffold
  section to the Shipping section
- Update their descriptions to reflect reality
- Update the test count total and per-crate breakdown where verifiable
- Fix the Critical Path section: mark `Interactive TUI` as DONE
- Update LOC/crate counts if cited

### 3. Fix `docs/INDEX.md` if it cites stale numbers

- Check whether the top-level INDEX cites 177K LOC, 18 crates, or ~85 routes
- If so, update to 322K LOC, 36 crates, 200+ routes
- Do NOT rewrite the "Current Framing" block -- that is AUD07's scope

### 4. Verify consistency

After edits, confirm that all three files agree on:
- roko-serve status (Shipping/Wired, 200+ routes)
- TUI status (Shipping/Wired, ratatui)
- LOC count (322K or "300K+")
- Crate count (36)
- Test count (3,761 or "3,700+")

## Write scope

- `docs/00-architecture/INDEX.md`
- `docs/STATUS.md`
- `docs/INDEX.md` (only if it cites stale numbers)

## Rules

1. **Only fix factual status and numbers.** Do not rewrite prose, restructure
   sections, or change architectural framing. That is later batches' scope.
2. **Use conservative numbers.** If unsure of exact count, use "200+" not "247"
   or "300+" not "322,088". Round to avoid false precision.
3. **Preserve the existing section structure.** Move items between tiers (e.g.,
   Scaffold -> Shipping) but do not add or remove sections.
4. **Do not touch any file outside the write scope.** Other docs will be fixed
   in AUD02-AUD08.
5. **Cross-reference against CLAUDE.md** for ground truth on what is wired.
   If CLAUDE.md and the audit disagree, note both and prefer the more
   conservative claim.

## Done when

- `docs/00-architecture/INDEX.md` no longer says serve is "not wired" or TUI
  is "text-mode only"
- `docs/STATUS.md` section 12 (Interfaces) is at least `Shipping` for TUI and
  HTTP API subsystems
- `docs/STATUS.md` no longer lists `HTTP server` or `Text dashboard` under
  Scaffold
- `docs/STATUS.md` Critical Path no longer lists TUI as a pending item
- All three docs agree on serve/TUI status and use consistent LOC/crate/route
  numbers
- No new sections or structural changes were introduced
- Final message lists every number changed, the old value, and the new value
