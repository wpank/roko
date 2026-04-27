# State of the World — Roko/Nunchi Project

> **Written**: 2026-04-26
> **Audience**: A fresh Claude Code session (or engineer) with zero prior context.
> **Purpose**: Capture everything needed to continue work without re-discovery.

---

## 1. What Is This Project?

**Nunchi** is a two-part system:
- **Roko** — An open-source Rust agent runtime (18 crates, ~177K LOC). Agents read PRDs,
  generate implementation plans, execute tasks via Claude/GPT/etc, validate with gates,
  and persist results. The core loop works end-to-end. It is self-hosting: roko develops itself.
- **Nunchi blockchain** — A sovereign EVM L1 for agent identity and on-chain knowledge
  (Phase 4, not yet wired into runtime).

**Who built it**: Will (solo founder). Previously built "mori" (the original orchestrator, 108K LOC)
at Uniswap/Bardo. Roko is the rewrite.

**Business context**: Series A pitch to a16z on May 6, 2026. The demo is a live terminal session
showing identity, routing, gates, knowledge-sharing, and crash recovery.

---

## 2. Codebase Layout

```
/Users/will/dev/nunchi/roko/roko/     <- Workspace root
  crates/
    roko-core/        Kernel: Signal type, 6 traits, config, tools, errors
    roko-cli/         CLI binary: all subcommands, TUI, orchestrator
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
  tmp/                Planning docs, specs, audit results (see Section 9)
  .roko/              Runtime data directory
    state/            Executor snapshots (executor.json)
    learn/            cascade-router.json, gate-thresholds.json, experiments.json, efficiency.jsonl
    episodes.jsonl    Agent turn recordings
    signals.jsonl     Signal log (currently empty)
    prd/              PRD storage
    research/         Research artifacts
```

**Key entry points**:
- CLI main: `crates/roko-cli/src/main.rs` (3,672 lines — reduced from 12,690)
- Orchestrator: `crates/roko-cli/src/orchestrate.rs` (21,653 lines — the god object, targeted for replacement)
- Command modules: `crates/roko-cli/src/commands/` (15 modules, all wired)
- Runner v2: `crates/roko-cli/src/runner/` (2,181 lines — built but NOT wired yet)

---

## 3. Current Branch: `wp-arch2`

All work is on branch `wp-arch2`. This branch has:
- All P0 dogfood fixes applied
- Config schema decomposition (schema.rs: 6,061 -> 929 lines)
- Cascade router refactor (cascade_router.rs: 5,197 -> 2,070 lines)
- main.rs decomposition (12,690 -> 3,672 lines, 15 command modules)
- Serve routes split (status/ and learning/ directories)
- Runner v2 module (built, not wired)
- 8 removed learn modules (4,808 LOC of dead code cleaned up)

**Other branches** (from parallel refactoring agents):
- `refactor/routes` — serve routes split (merged into wp-arch2)
- `refactor/cascade` — cascade router split (merged)
- `claude/migration-run-*` — automated migration agent runs (some merged, some not)

**Never push to main without asking Will.**

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
builder), episode logging, learning feedback, adaptive thresholds, experiments — **wired and working**.

---

## 5. What's Broken / Incomplete

### P0 — All Fixed
Every P0 blocker was resolved on `wp-arch2`. See `tmp/dogfood/00-INDEX.md` for the full list.

### P1 — Degrades Experience (7 open)

| ID | Issue | Impact |
|----|-------|--------|
| **#8** | TOML parse fails on markdown fences | Enrichment verify step fails when LLMs wrap output in ```toml |
| **#9** | Enrichment timeouts too short (120s) | Plans with 16+ files timeout |
| **M1** | No streaming agent output | TUI is blind during agent execution. Batch-only. |
| **M2** | Model shows "-" in TUI | Model name not in AgentSpawned event |
| **M3** | Tokens/cost show "0k/$0.00" | No streaming token counts |
| **F5** | Memory leak — 9.5GB RSS in 17 min | Likely enrichment artifact strings |
| **F9** | TUI log garbled | tracing vs raw terminal conflict |

**M1 is the big one.** The TUI sees nothing during agent runs because output is captured batch-only.
Mori solved this with `--stream-json` per-line parsing. Runner v2 was designed to fix this.

### P2 — Missing Features (5 open)

| ID | Issue |
|----|-------|
| **#4** | No codex backend (can't use gpt-5.4) |
| **#11** | No `/api/plans/:id` and `/api/plans/:id/tasks` routes |
| **#12** | No knowledge HTTP endpoint |
| **#13** | No executor state HTTP endpoint |
| **#17** | `/api/learn/router` returns 404 |

### P3 — Polish (5 open)
signals.jsonl always empty, TUI log useless, learn/ files stale, no worktree isolation.

---

## 6. The Eight Migration Plans

Located in `tmp/unified-migration-runner/`. These are self-contained implementation prompts
designed to be given to Claude Code agents. Current status:

### ARCHIVE-READY (fully implemented)

| Plan | What it did | Result |
|------|-------------|--------|
| **CONFIG-SCHEMA-DECOMPOSITION.md** | Split schema.rs (6,061 lines) into 12 focused config modules | schema.rs now 929 lines. All 72 config tests pass. |
| **CASCADE-ROUTER-REFACTOR.md** | Split cascade_router.rs (5,197 lines) into cascade/ submodules | cascade_router.rs now 2,070 lines. 4 submodules created. Minor items (arms.rs, explain.rs, ModelSelector trait) deferred as optional. |
| **REFACTORING-PROMPT.md** | Sequential execution guide for Tracks A-E | Coordination doc. A-D done, E (Cell) not started. |
| **REFACTORING-PROMPT-PARALLEL.md** | Parallel team execution guide for Tracks A-E | Same as above but for multi-agent parallel execution. |

### STILL NEEDS WORK

| Plan | Status | What's Left |
|------|--------|-------------|
| **CELL-TRAIT-AND-RENAMES.md** | **0% done** | Major cross-crate rename: 6 traits (Substrate->Store, Scorer->Score, Gate->Verify, Router->Route, Composer->Compose, Policy->React), new Cell trait, 3 new protocol stubs (Observe, Connect, Trigger), backwards compat. ~87 impl blocks to update. |
| **DEMURRAGE-AND-TIERS.md** | **~60% done** | Rate law + knowledge store + config done. Missing: tier progression logic (evaluate_tier/maybe_promote), Engram fields (implemented on KnowledgeEntry instead), FileSubstrate wiring. |
| **RUNNER-V2-IMPLEMENTATION.md** | **95% built, 0% wired** | All 10 runner/ files exist (2,181 LOC). But `roko plan run` still uses legacy PlanRunner from orchestrate.rs. Missing: CLI wiring, `--v2` flag or default switch, orchestrate.rs deprecation. |
| **SERVE-ROUTES-CONSOLIDATION.md** | **67% done** | status/ and learning/ split done. plans.rs split NOT done. Knowledge HTTP endpoints not added. |

---

## 7. The Runner v2 Problem (Most Important Thread)

**orchestrate.rs** is a 21,653-line god object with 250+ methods. It works but is fragile:
- Agent output is batch-only (TUI sees nothing during execution)
- All persistence buffered in memory (crash = data loss)
- Enrichment pipeline overwrites user files
- Plan discovery confuses enrichment artifacts with plans

**runner/** (2,181 lines) was built to replace it:
- Streams agent output via `--stream-json`
- Flushes persistence after every task
- Publishes DashboardEvents for TUI
- Loads tasks.toml directly (no magic discovery)
- Clean event loop with tokio::select!

**The gap**: runner/ exists but isn't called. `commands/plan.rs:200-208` still instantiates `PlanRunner`
from orchestrate.rs. Someone needs to:
1. Add `--v2` flag (or make it default)
2. Call `runner::event_loop::run()` instead of `PlanRunner::run()`
3. Test end-to-end
4. Eventually deprecate orchestrate.rs

This is the single highest-impact unfinished work. It fixes M1, M2, M3, F5, and F9 by design.

---

## 8. The May 6 Demo

**What**: 5-minute live terminal demo for a16z Series A pitch.
**Spec**: `tmp/dogfood/09-MAY6-DEMO-BUILD.md`

Five commands to demo:
1. `nunchi agents list` — show registered agents with identity/model/status
2. `nunchi audit deployment payments-svc` — 8-step audit showing identity, routing, gates, knowledge
3. Ctrl+C at step 5 (pre-seeded failure)
4. `nunchi resume run_4823` — resume from checkpoint
5. `nunchi replay run_4823` — stream JSON audit trail

Key requirements:
- `nunchi` CLI wrapper (thin shell around `cargo run -p roko-cli`)
- LLM cache pre-warming (demo runs offline, deterministically)
- Backup tiers: asciinema recording, MP4, screenshots
- Clack-style terminal formatting (special symbols, not emoji)

See `tmp/dogfood/12-DECK-AND-MEMO.md` for the 13-slide deck and 2,000-word pre-read memo checklist.

---

## 9. Guide to the tmp/ Directory

The `tmp/` directory has ~80 subdirectories and many files. Here's what matters:

### Active / Authoritative

| Directory | What | Read This |
|-----------|------|-----------|
| `tmp/dogfood/` | Known bugs, demo build, runtime fixes | `00-INDEX.md` (master checklist) |
| `tmp/unified/` | Unified architecture spec (22 docs) | The canonical spec for Phase 1-3 migration |
| `tmp/unified-depth/` | Deep research backing the unified spec | Algorithms, citations, domain patterns |
| `tmp/unified-migration-runner/` | Implementation plans for current refactoring | This file you're reading |
| `tmp/unified-migration/` | Phase 1-3 migration roadmap | `02-PHASE-1-KERNEL.md` for next big work |
| `tmp/learnings2/` | Strategy docs (Series A, market, positioning) | Investor-facing narrative |
| `tmp/research/` | Recent research outputs | Various topics |
| `tmp/architecture/` | Architecture decisions (18+ docs) | `18-roadmap.md` for Phases 1-8 |
| `tmp/workflow/` | Workflow engine design (Workspace/Module/Workflow/Trigger) | Future generalized orchestration |
| `tmp/04-21-26/` | April 21 deep session (arenas, HDC, knowledge publishing) | `09-unified-narrative.md` for the full vision |

### Historical / Superseded (still useful as reference)

| Directory | What | Notes |
|-----------|------|-------|
| `tmp/MASTER-PLAN.md` | Original 180K-word master plan | Superseded by unified spec |
| `tmp/MORI-PARITY-GAP-ANALYSIS.md` | 107K-word gap analysis | Historical reference |
| `tmp/implementation-plans/` | Earlier implementation plans | Partially superseded by unified-migration-runner |
| `tmp/architecture-plans/` | Earlier architecture plans | Superseded by architecture/ |
| `tmp/docs-gaps/`, `docs-parity/`, `docs-parity2/`, `docs-parity-meta/` | Documentation gap analyses | Multiple iterations, mostly historical |
| `tmp/tui/`, `tui-parity/` | TUI implementation docs | TUI is done; historical |
| `tmp/refinements/`, `refinements-audit/`, `refinements-runner/` | Earlier refinement passes | Historical |
| `tmp/prd-enhance-logs/` | 76 PRD enhancement run logs | Historical output |
| `tmp/run-*.sh` | Batch execution scripts | Historical automation |
| `tmp/logs/` | 533 log files | Historical execution logs |

### The Supersession Chain

Understanding how docs evolved:
1. `MASTER-PLAN.md` + `MORI-PARITY-GAP-ANALYSIS.md` (April 8-10) — initial audit
2. `implementation-plans/` + `architecture-plans/` (April 10-17) — first generation plans
3. `TODO/` + `ux-followup-runner/` (April 15-20) — follow-up work tracking
4. `roko-architecture-redesign.md` + `v2` (April 24) — architectural vision crystallization
5. `unified/` + `unified-depth/` (April 25-26) — canonical unified spec
6. `unified-migration/` + `unified-migration-runner/` (April 26) — implementation from spec
7. `dogfood/` (April 26) — real-world testing findings

**When in doubt**, `tmp/unified/` is the architectural authority and `tmp/dogfood/` is the
practical authority for what's broken.

---

## 10. The Unified Architecture (Where This Is Heading)

The current architecture uses `Engram` as the universal data type and 6 traits:
Substrate, Scorer, Gate, Router, Composer, Policy.

The unified spec (`tmp/unified/`) replaces this with:

### Three Fundamentals
- **Signal** — Durable, content-addressed, HDC-fingerprinted, decaying (replaces Engram)
- **Pulse** — Ephemeral, ring-buffered on Bus (new)
- **Graph** — Universal composition (workflows, pipelines, dream cycles — all one runtime)

### Nine Protocols (every "Cell" implements a subset)
1. **Store** (was Substrate) — Read/write Signals
2. **Score** (was Scorer) — Evaluate quality
3. **Verify** (was Gate) — Gate pipeline
4. **Route** (was Router) — Model/agent selection
5. **Compose** (was Composer) — Prompt assembly
6. **React** (was Policy) — Agent dispatch
7. **Observe** (new) — Read-only monitoring
8. **Connect** (new) — External I/O lifecycle
9. **Trigger** (new) — Event-driven activation

### Demurrage (Economic Decay)
Signals have a `balance` that decreases over time. Retrieving/citing a Signal reinforces it.
When balance hits zero, the Signal archives to cold storage. This forces the system to
prioritize actively useful knowledge. Partially implemented on KnowledgeEntry but not on
Engram/Signal itself.

### Four Migration Phases
1. **Phase 0** — Dead code cleanup, scaffolding (mostly done)
2. **Phase 1** — Pulse/Bus, Cell trait, protocol renames, demurrage, heuristics
3. **Phase 2** — Graph engine, CognitiveWorkspace, Surfaces, Marketplace
4. **Phase 3** — L4 self-evolution, on-chain integration, arenas

**Current position**: Between Phase 0 and Phase 1. The refactoring (plans in Section 6)
is Phase 0 work. Cell trait + renames + demurrage are Phase 1.

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

The workspace compiles cleanly on `wp-arch2`. All tests pass.

---

## 12. Immediate Next Actions (Priority Order)

### For the May 6 demo
1. Build `nunchi` CLI wrapper
2. Implement `nunchi agents list` with Clack-style formatting
3. Implement `nunchi audit` (scripted or wired)
4. Pre-warm LLM cache
5. Record backup tiers (asciinema + MP4)
6. Build 13-slide deck + 2,000-word memo

### For runtime quality
1. Wire runner v2 into CLI (fixes streaming, persistence, TUI)
2. Strip markdown fences from TOML parsing (#8)
3. Embed model in AgentSpawned event (M2)
4. Emit EfficiencyUpdate after dispatch (M3)
5. Investigate memory leak (F5)

### For architecture alignment
1. Cell trait + protocol renames (CELL-TRAIT-AND-RENAMES.md)
2. Finish demurrage tier progression
3. Complete serve routes split (plans.rs)

### Archive completed plans
Move these to `tmp/unified-migration-runner/archive/`:
- CONFIG-SCHEMA-DECOMPOSITION.md
- CASCADE-ROUTER-REFACTOR.md
- REFACTORING-PROMPT.md
- REFACTORING-PROMPT-PARALLEL.md

---

## 13. Key Decisions & Design Rationale

### Why orchestrate.rs is 21K lines
Parallel development across many sessions. Each session added features without refactoring.
The file grew from ~5K to 21K over 3 weeks. Runner v2 is the clean replacement.

### Why demurrage went on KnowledgeEntry instead of Engram
The Engram type is used everywhere (signals, episodes, all substrates). Adding economic
fields to it would have been a much larger change. KnowledgeEntry in roko-neuro is the
right scope for now — knowledge is what needs decay/reinforcement. Universal signal
demurrage can come later when Engram is renamed to Signal.

### Why Cell trait renames aren't done yet
It's an 87-impl-block cross-crate rename that touches every crate. It depends on the
refactoring (Sections A-D of the plans) being done first to minimize merge conflicts.
The refactoring is now done, so Cell renames can proceed.

### Why runner v2 isn't wired
It was built during a deep audit session (commit cc2a3cfb) but the final wiring step
(changing `commands/plan.rs` to call the new runner) wasn't done because it needs
end-to-end testing with a real plan run. The old PlanRunner still works, so this is
a "wire, don't build" problem.

### Why there are so many tmp/ directories
Each Claude Code session creates planning docs. Sessions don't share context, so each
one generates its own analysis. The unified/ spec was explicitly written to end this —
it's the canonical reference that all future sessions should use instead of generating
new analysis docs.

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
| Mori parity checklist (1,253 items) | `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/MORI-PARITY-CHECKLIST.md` |
| Mistakes learned (30+ entries) | `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/MISTAKES-LEARNED.md` |
