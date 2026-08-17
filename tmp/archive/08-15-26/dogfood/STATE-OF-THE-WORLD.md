# State of the World -- Roko/Nunchi Project

> **Written**: 2026-04-26
> **Last updated**: 2026-08-13
> **Audience**: A fresh Claude Code session (or engineer) with zero prior context.
> **Purpose**: Capture everything needed to continue work without re-discovery.

## Current status (2026-08-13)

> **This document was originally written 2026-04-26 and has been updated with corrections.**
> Several sections described issues and code that no longer exist. Key changes since original writing:
>
> - **orchestrate.rs (21K lines) has been DELETED.** Runner v2 in `crates/roko-cli/src/runner/event_loop.rs`
>   replaced it entirely. All references to orchestrate.rs below are historical.
> - **event_loop.rs is now ~19,846 lines** and is the new god-object concern.
> - **All P0, P1, and P2 issues are RESOLVED.** Only 4 items remain open (3 P3 + Phase E).
> - **Runner v2 is the default** for all `plan run` invocations. Not just `--approval` mode.
> - **Engram-to-Signal rename: DONE (2026-08-12).** `pub type Signal = Engram` alias landed
>   in `crates/roko-core/src/engram.rs` with a `signal.rs` re-export module. New code can use
>   `Signal`. The underlying struct is still `Engram` and `engrams.jsonl` is still on disk (~29
>   files). Full struct rename deferred to Phase 1 (Cell trait).
> - **contextual_bandit.rs came back as dead code.** Removed April 2026 (1,372 LOC) but
>   re-added by a batch agent run. Only referenced from one test. No production callers.
> - **All 6 critical dogfood fixes are RESOLVED** (force_shutdown, persistence, efficiency,
>   model routing, implementation dispatch, test compilation).
> - **The May 6 a16z demo date has passed.** Sections 8 and 12 about demo prep are historical.
>
> **What is dogfooding?** Roko is designed to develop itself -- it reads PRDs, generates
> plans, dispatches LLM agents, validates results, and persists everything. "Dogfooding"
> means actually running this workflow and recording what breaks. This file and the
> `tmp/dogfood/` folder document those findings.

---

## 1. What Is This Project?

**Nunchi** is a two-part system:
- **Roko** -- An open-source Rust agent runtime (18 crates, ~177K LOC). Agents read PRDs,
  generate implementation plans, execute tasks via Claude/GPT/etc, validate with gates,
  and persist results. The core loop works end-to-end. It is self-hosting: roko develops itself.
- **Nunchi blockchain** -- A sovereign EVM L1 for agent identity and on-chain knowledge
  (Phase 4, not yet wired into runtime).

**Who built it**: Will (solo founder). Previously built "mori" (the original orchestrator, 108K LOC)
at Uniswap/Bardo. Roko is the rewrite.

---

## 2. Codebase Layout

```
/Users/will/dev/nunchi/roko/roko/     <- Workspace root
  crates/
    roko-core/        Kernel: Signal type, 6 traits, config, tools, errors
    roko-cli/         CLI binary: all subcommands, TUI, runner
    roko-agent/       5+ LLM backends, dispatch, MCP, tool loop, safety
    roko-agent-server/ Per-agent HTTP sidecar (13 routes)
    roko-serve/       HTTP control plane (~85 routes on :6677)
    roko-orchestrator/ Plan DAG, parallel executor
    roko-gate/        11 gates, 7-rung pipeline
    roko-compose/     Prompt assembly, 9 templates
    roko-learn/       Episodes, cascade router, experiments, efficiency
    roko-neuro/       Durable knowledge store, distillation
    roko-fs/          File storage (JSONL substrate)
    roko-std/         Defaults, 19 builtin tools, mock dispatcher
    roko-runtime/     ProcessSupervisor, event bus, cancellation
    roko-primitives/  HDC vectors, tier routing
    roko-dreams/      Offline consolidation (hypnagogia, imagination)
    roko-daimon/      Affect engine, somatic markers
    roko-conductor/   10 watchers, circuit breaker
    roko-chain/       Chain witness primitives (Phase 2+)
    roko-mcp-code/    Code-intelligence MCP server
    roko-index/       Parser + graph + HDC indexing
  tmp/                Planning docs, specs, audit results
  .roko/              Runtime data directory
    state/            Executor snapshots (executor.json)
    learn/            cascade-router.json, gate-thresholds.json, experiments.json, efficiency.jsonl
    episodes.jsonl    Agent turn recordings
    signals.jsonl     Signal log (currently empty -- see S4 in 00-INDEX.md)
    prd/              PRD storage
    research/         Research artifacts
```

**Key entry points**:
- CLI main: `crates/roko-cli/src/main.rs`
- Plan runner (CURRENT): `crates/roko-cli/src/runner/event_loop.rs` (~19,846 lines -- the current god-object)
- Runner module: `crates/roko-cli/src/runner/` (25 files)
- Command modules: `crates/roko-cli/src/commands/` (15 modules, all wired)

> **DELETED**: `crates/roko-cli/src/orchestrate.rs` -- the original 21K-line god object.
> Runner v2 replaced it entirely. References to orchestrate.rs throughout this document
> are historical.

---

## 3. Current Branch

> **Note**: The `wp-arch2` branch described in the original version of this document has
> been merged. Work continues on feature branches off `main`. See the current git branch
> for active work.

---

## 4. What Works Today

The self-hosting workflow is fully operational:

```bash
roko prd idea "Some work item"            # Capture idea
roko prd draft new "slug"                 # Agent writes PRD
roko research enhance-prd slug            # Research enriches PRD
roko prd plan slug                        # Agent generates tasks.toml
roko plan run plans/                      # Execute (agents + gates + persistence)
roko plan run plans/ --resume .roko/state/executor.json  # Resume if interrupted
roko dashboard                            # Interactive TUI (ratatui, F1-F7)
roko serve                                # HTTP control plane on :6677
```

All of: plan discovery, DAG execution, agent dispatch (5+ backends), gate pipeline (11 gates),
session persistence, MCP passthrough, model routing (cascade router), prompt assembly (9-layer
builder), episode logging, learning feedback, adaptive thresholds, experiments, streaming
agent output, TUI live updates -- **wired and working**.

---

## 5. What's Broken / Incomplete

### P0, P1, P2 -- ALL RESOLVED

Every P0, P1, and P2 issue from the original dogfood runs has been fixed. See
`tmp/dogfood/00-INDEX.md` for the full checklist with resolution notes.

### P3 -- Polish / Tech Debt (3 OPEN) + Runner v2 Phase E (1 OPEN)

| ID | Issue | Status |
|----|-------|--------|
| **#15** | Enrichment artifacts mostly empty/minimal | OPEN (moot with skip_enrichment, low priority) |
| **S4** | signals.jsonl stays at 0 lines | OPEN (conductor writes to engrams.jsonl; Signal alias exists but file path not renamed) |
| **S7** | learn/ files stale | OPEN (runner v2 doesn't update cascade-router.json or gate-thresholds.json) |
| **Phase E** | Runner v2 spec alignment (type renames, Activity recording) | OPEN |

### Architectural concerns

| Concern | Detail |
|---------|--------|
| **event_loop.rs god-object** | ~19,846 lines, same problem orchestrate.rs had. Needs decomposition. |
| **Engram-to-Signal rename** | PARTIALLY DONE. `pub type Signal = Engram` alias landed (2026-08-12). Struct still `Engram`, file still `engrams.jsonl`. Full rename deferred to Phase 1. |
| **contextual_bandit.rs dead code** | 1,372 LOC, removed April 2026, re-added by batch agent run. Only used in one test. Should be removed again. |
| **CognitiveWorkspace not wired** | Types + builder exist but runner never produces one. |
| **ExtensionChain always empty** | Hooks called at 5 points but chain has no extensions loaded. |

---

## 6. The Migration Plans

Located in `tmp/unified-migration-runner/`. These are self-contained implementation prompts
designed to be given to Claude Code agents.

### COMPLETED

| Plan | What it did | Result |
|------|-------------|--------|
| **CONFIG-SCHEMA-DECOMPOSITION.md** | Split schema.rs into 12 focused config modules | schema.rs reduced from 6,061 to 929 lines |
| **CASCADE-ROUTER-REFACTOR.md** | Split cascade_router.rs into cascade/ submodules | cascade_router.rs reduced from 5,197 to 2,070 lines |
| **RUNNER-V2-IMPLEMENTATION.md** | Build event-driven runner to replace orchestrate.rs | DONE -- runner v2 is default, orchestrate.rs deleted |
| **SERVE-ROUTES-CONSOLIDATION.md** | Split serve routes into subdirectories | Partially done (status/ and learning/ split, plans.rs not split) |

### STILL NEEDS WORK

| Plan | Status | What's Left |
|------|--------|-------------|
| **CELL-TRAIT-AND-RENAMES.md** | 0% done | Cross-crate rename: 6 traits, new Cell trait, 3 new protocol stubs. ~87 impl blocks. |
| **DEMURRAGE-AND-TIERS.md** | ~60% done | Rate law + knowledge store + config done. Missing: tier progression, Engram fields, FileSubstrate wiring. |

---

## 7. The Runner v2 Story (Historical Context)

> **This section is historical.** orchestrate.rs no longer exists. Runner v2 won.

**orchestrate.rs** was a 21,653-line god object with 250+ methods. It worked but was fragile:
- Agent output was batch-only (TUI saw nothing during execution)
- All persistence buffered in memory (crash = data loss)
- Enrichment pipeline overwrote user files
- Plan discovery confused enrichment artifacts with plans

**runner/** (originally 2,181 lines) was built to replace it:
- Streams agent output via `--output-format stream-json`
- Flushes persistence after every task
- Publishes DashboardEvents for TUI
- Loads tasks.toml directly (no magic discovery)
- Clean event loop with tokio::select!

**What happened**: Runner v2 was wired as the default for all `plan run` invocations.
orchestrate.rs was then deleted. However, event_loop.rs has since grown to ~19,846 lines,
reproducing the same god-object problem. The runner module now has 25 files but most logic
lives in event_loop.rs.

---

## 8. The May 6 Demo (HISTORICAL)

> **This section is historical.** The May 6, 2026 date has passed. Keeping for reference
> on demo architecture decisions.

Five commands were planned for a 5-minute live terminal demo for a16z:
1. `nunchi agents list` -- show registered agents
2. `nunchi audit deployment payments-svc` -- 8-step audit showing identity, routing, gates, knowledge
3. Ctrl+C at step 5 (pre-seeded failure)
4. `nunchi resume run_4823` -- resume from checkpoint
5. `nunchi replay run_4823` -- stream JSON audit trail

See `09-MAY6-DEMO-BUILD.md` for the full spec. See `12-DECK-AND-MEMO.md` for deck/memo checklist.

---

## 9. Guide to the tmp/ Directory

The `tmp/` directory has ~80 subdirectories and many files. Here's what matters:

### Active / Authoritative

| Directory | What |
|-----------|------|
| `tmp/dogfood/` | This folder. Dogfood findings, fixes, demo prep. |
| `tmp/unified/` | Protocol specification v2.0. The spec authority. |
| `tmp/unified-depth/` | Deep algorithmic backing for each section of unified spec. |
| `tmp/architecture/` | 21 implementation-focused specs. |
| `tmp/learnings2/` | 11-doc briefing set for investors and team onboarding. |

### Historical / safe to ignore

Most other `tmp/` directories are from earlier iterations, superseded by the unified spec.
See CONTEXT.md for a more concise navigation guide.

---

## 10. The Unified Architecture (Where This Is Heading)

The current architecture uses `Engram` as the universal data type and 6 traits:
Substrate, Scorer, Gate, Router, Composer, Policy.

> **Rename status (2026-08-13)**: PARTIALLY DONE. `pub type Signal = Engram` alias landed
> on 2026-08-12 in `crates/roko-core/src/engram.rs` with a `signal.rs` re-export module.
> New code can use `Signal` everywhere. The underlying struct is still `Engram` and
> `engrams.jsonl` is still the file name on disk (~29 files). Full struct rename is part
> of Phase 1 (Cell trait + protocol renames).

The unified spec (`tmp/unified/`) replaces this with:

### Three Fundamentals
- **Signal** -- Durable, content-addressed, HDC-fingerprinted, decaying (replaces Engram)
- **Pulse** -- Ephemeral, ring-buffered on Bus (new)
- **Graph** -- Universal composition (workflows, pipelines, dream cycles -- all one runtime)

### Nine Protocols (every "Cell" implements a subset)
1. **Store** (was Substrate) -- Read/write Signals
2. **Score** (was Scorer) -- Evaluate quality
3. **Verify** (was Gate) -- Gate pipeline
4. **Route** (was Router) -- Model/agent selection
5. **Compose** (was Composer) -- Prompt assembly
6. **React** (was Policy) -- Agent dispatch
7. **Observe** (new) -- Read-only monitoring
8. **Connect** (new) -- External I/O lifecycle
9. **Trigger** (new) -- Event-driven activation

### Four Migration Phases
1. **Phase 0** -- Dead code cleanup, scaffolding (DONE)
2. **Phase 1** -- Pulse/Bus, Cell trait, protocol renames, demurrage, heuristics (NOT STARTED)
3. **Phase 2** -- Graph engine, CognitiveWorkspace, Surfaces, Marketplace
4. **Phase 3** -- L4 self-evolution, on-chain integration, arenas

**Current position (2026-08-13)**: Phase 0 is complete. Phase 1 (Cell trait + renames)
has not started. The Engram-to-Signal rename is part of Phase 1.

---

## 11. Build & Test

```bash
cd /Users/will/dev/nunchi/roko/roko
rustup update stable          # Need 1.91+ for alloy deps
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --no-deps -- -D warnings
cargo +nightly fmt --all      # Nightly for formatting (matches CI)
```

---

## 12. Immediate Next Actions (Priority Order)

> **Updated 2026-08-13.** The original "next actions" list was written for pre-May-6 demo prep
> and is now historical. Current priorities below.

### For runtime quality
1. Decompose event_loop.rs (~19,846 lines) -- extract subsystems into separate runner/ files
2. Add CascadeRouter persistence to runner v2 (S7)
3. Add gate-threshold persistence to runner v2 (S7)
4. Wire replan-on-gate-failure in runner v2
5. Remove contextual_bandit.rs dead code (again) -- 1,372 LOC, re-added by batch agent

### For architecture alignment
1. Complete Engram-to-Signal struct rename (alias landed 2026-08-12, full rename is Phase 1)
2. Cell trait + protocol renames (CELL-TRAIT-AND-RENAMES.md -- Phase 1)
3. Finish demurrage tier progression

### Low priority
1. Fix signals.jsonl dead path (S4) -- will resolve when engrams.jsonl is renamed to signals.jsonl
2. Wire CognitiveWorkspace
3. Add extension loader for ExtensionChain

---

## 13. Key Decisions & Design Rationale

### Why orchestrate.rs existed (and why it was deleted)

Parallel development across many sessions. Each session added features without refactoring.
The file grew from ~5K to 21K over 3 weeks. Runner v2 was built as the clean replacement.
orchestrate.rs was deleted after runner v2 was made the default. However, event_loop.rs
has now grown to ~19,846 lines and needs the same treatment.

### Why demurrage went on KnowledgeEntry instead of Engram

The Engram type is used everywhere (signals, episodes, all substrates). Adding economic
fields to it would have been a much larger change. KnowledgeEntry in roko-neuro is the
right scope for now. Universal signal demurrage can come when Engram is renamed to Signal.

### Why Cell trait renames aren't done yet

It's an 87-impl-block cross-crate rename that touches every crate. Depends on Phase 0
cleanup being done first (it is). Ready to proceed.

### Why there are so many tmp/ directories

Each Claude Code session creates planning docs. Sessions don't share context, so each
one generates its own analysis. The unified/ spec was written to end this -- it's the
canonical reference that all future sessions should use instead of generating new analysis docs.

---

## 14. Reference Material (Read-Only, Don't Modify)

| What | Path |
|------|------|
| Mori orchestrator (reference impl) | `/Users/will/dev/uniswap/bardo/apps/mori/` |
| Mori agent connection (reference spawn) | `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs:2444-2620` |
| Original 36 bardo crates | `/Users/will/dev/uniswap/bardo/crates/` |
| Mori plans (171 plans) | `/Users/will/dev/uniswap/bardo/.mori/plans/` |
| PRD documents (359 files) | `/Users/will/dev/nunchi/roko/bardo-backup/prd/` |
| Roko progress docs (140+ files) | `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/` |
