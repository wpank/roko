# Dogfood Context — Updated 2026-04-26 (Session 2)

> This document is the "state of the world" for someone picking up this work
> with zero context. Read this first, then dig into the files it references.

---

## What is Roko

Roko is a Rust toolkit (18 crates, ~177K LOC) for building agents that build themselves. It reads PRDs, generates implementation plans, executes tasks via Claude/Codex agents, validates results through gate pipelines, and persists everything. The core entry point is `crates/roko-cli/src/main.rs`. The orchestration engine lives in `crates/roko-cli/src/orchestrate.rs` (~21K lines).

The self-hosting workflow:
```
roko prd idea "..." → roko prd draft → roko prd plan <slug> → roko plan run plans/ → roko dashboard
```

## Branch: `wp-arch2`

All work is on this branch. Multiple Claude sessions have been working on it. The git status has significant unstaged changes across roko-cli, roko-core, roko-learn, and roko-serve.

---

## What happened across sessions

### Session 1 (earlier on 2026-04-26)

Killed a hung roko process (11.5GB RAM). Audited dogfood findings from 3 real plan-runner executions. Fixed 6 critical issues (force_shutdown self-kill, no executor.json persistence, efficiency events not flushed, model fallback to haiku, implementation phase never dispatching, test compilation). Created `07-consolidated-open-issues.md` with all remaining issues. Updated `00-INDEX.md` to 21/56 done.

### Session 2 (this session, 2026-04-26)

#### 1. Audited the roko-trustworthy runner

The `roko-trustworthy` runner was a 24-batch (RT00-RT23) overnight Codex run that added trustworthiness infrastructure to the codebase. All 24 batches passed. The implementations are real -- well-typed, tested, mostly wired. Found 6 gaps:

| Gap | Status | Detail |
|-----|--------|--------|
| `ContextualBanditPolicy` dead code | **Removed** | 1,372 LOC. `UcbBandit` in gateway.rs handles model selection instead. |
| `CognitiveWorkspace` not wired | Open | Types + builder exist in roko-core/roko-compose but orchestrate.rs never produces one. |
| `ExtensionChain` always empty | Open | Hooks called at 5 points in orchestrate.rs but chain has no extensions (no loader/factory). |
| Warm-agent pooling absent | Open | `reuse_policy_id` field exists but no process reuse. Deferred by design. |
| `prd_prompt.rs` bardo paths | Open | Hardcoded `/Users/will/dev/uniswap/bardo/prd/` in live agent prompts (lines 152-154). |
| E2E test is `#[ignore]` | Open | Needs mock fixture. Acceptable. |

Confirmed several AUDIT.md issues were already fixed:
- CascadeRouter cached via RwLock (not per-request disk I/O)
- Config uses ArcSwap (lock-free reads)
- RoutingContext accepts caller hints (not hardcoded)
- KnowledgeAdmissionController wired in production

The runner folder was archived to `tmp/archive/roko-trustworthy/`.

#### 2. Removed 4,808 lines of dead code from roko-learn

8 modules with zero production callers:

| Module | Lines | Why removed |
|--------|-------|-------------|
| `contextual_bandit.rs` | 1,372 | Superseded by `UcbBandit` in gateway.rs |
| `bandit_research.rs` | 862 | Doc-parity shells, not production |
| `causal.rs` | 699 | TA-08 theoretical, no signal pipeline |
| `shapley.rs` | 518 | P1-08, no multi-agent credit surface |
| `resonant_patterns.rs` | 373 | TA-09 theoretical, no integration point |
| `kalman.rs` | 354 | P2-10, no oracle pipeline |
| `adversarial.rs` | 321 | TA-10, no signal validation surface |
| `signal_metabolism.rs` | 309 | TA-07, no metabolism runtime |

Reintegration notes: `tmp/backlog/removed-learn-modules.md`. Recoverable from git history.

**Known issue:** A linter/IDE hook keeps reverting `crates/roko-learn/src/lib.rs` to add back the deleted module declarations. The .rs files are gone from disk. If you see compile errors, remove these `pub mod` lines from lib.rs: `adversarial`, `bandit_research`, `causal`, `contextual_bandit`, `kalman`, `resonant_patterns`, `shapley`, `signal_metabolism`.

---

## The dogfood folder (tmp/dogfood/)

This folder is the QA log from dogfooding Roko -- actually running `roko plan run` and recording what breaks. Comes from 3 real executions.

### Read in this order

| # | File | What | Current? |
|---|------|------|----------|
| 1 | **00-INDEX.md** | Master checklist, 56 items. **Start here.** | YES |
| 2 | **07-consolidated-open-issues.md** | All unresolved issues (C1-C6, H1-H4, M1-M5) | YES |
| 3 | **05-mori-vs-roko-agent-wiring.md** | Root cause: Roko batches agent output, Mori streamed. Why TUI is blind. | KEY DOCUMENT |
| 4 | **10-RUNTIME-FIXES.md** | Fix batches by impact (6 batches) | YES |
| 5 | **08-statehub-tui-audit.md** | 7 disconnects in StateHub→TUI data flow, 9 concrete fixes | YES |
| 6 | **07-orchestrate-analysis.md** | 21K-line god object decomposition plan | Superseded by Runner v2 |
| 7 | **09-MAY6-DEMO-BUILD.md** | Demo spec for May 6 a16z pitch | ACTIVE |
| 8 | **12-DECK-AND-MEMO.md** | 13-slide deck + memo spec | ACTIVE |
| 9 | **11-LANDING-PAGE-UPDATES.md** | nunchi.network cleanup | ACTIVE |
| 10 | **archive/resolved-2026-04-26.md** | 7+ fixed issues (TUI crash, skip_enrichment, health endpoint, etc.) | Historical |
| 11 | **CONTEXT.md** | This file | YES |

Files 01-04, 06 are historical run logs. Still useful for root cause context but findings are consolidated in 07.

---

## The broader tmp/ landscape

~80 items have accumulated. Here's what matters:

### Current authority documents (read these)

| Folder | What |
|--------|------|
| `tmp/learnings2/` | 11-doc briefing set: architecture, strategy, business, research, competitive intel, roadmap. For investors and team onboarding. |
| `tmp/unified/` | Protocol specification v2.0. 3 fundamentals (Signal/Cell/Graph), 9 protocols, 10 specializations. The spec authority. |
| `tmp/unified-depth/` | Deep algorithmic backing for each section of unified spec. |
| `tmp/architecture/` | 21 implementation-focused specs (gateway, auth, knowledge, groups, arenas, dashboard). |
| `tmp/dogfood/` | This folder. Dogfood findings, fixes, demo prep. |

### Implementation roadmaps (active)

| Folder | What |
|--------|------|
| `tmp/unified-migration/` | 4-phase migration checklist: current arch → unified spec |
| `tmp/unified-migration-runner/` | Agent-driven refactoring infrastructure (prompts, context packs, runner script) |
| `tmp/backlog/` | Removed code documentation with reintegration notes |

### Detailed specs (active but secondary)

| Folder | What |
|--------|------|
| `tmp/workflow/` | 12 PRDs for unified workflow subsystem |
| `tmp/visual-gate2/` | 10 PRDs for unified evaluation framework (supersedes visual-gate/) |
| `tmp/deck/` | Series A pitch materials + screenshots |
| `tmp/research/` | 15 research docs feeding strategy |
| `tmp/depth/` | Behavioral spec v1.0 (superseded by unified/ v2.0) |

### Historical / safe to ignore

`tmp/learnings/` (superseded by learnings2), `tmp/docs-gaps/`, `tmp/docs-parity*/`, `tmp/refinements*/`, `tmp/tui*/`, `tmp/prd-*/`, `tmp/run-*.sh`, `tmp/logs/`, `tmp/MASTER-*.md`, `tmp/MORI-*.md`, `tmp/sdb-spec/`, `tmp/ux*/`, `tmp/04-*-26/`, `tmp/agent-registry/`, `tmp/integrate-prds/`, `tmp/contracts/`, `tmp/defi/`, `tmp/a2a/`, root-level `.md` files (session notes, old designs, PR drafts).

---

## Open threads (priority order)

### Thread 1: Dogfood runtime issues
- **Where:** `tmp/dogfood/07-consolidated-open-issues.md`, `00-INDEX.md`
- **Status:** All P0s fixed. 6 critical + 4 high issues remain.
- **Key blocker:** Agent output pipeline is batch-only, not streaming. TUI is blind during runs. See `05-mori-vs-roko-agent-wiring.md`.
- **Fix roadmap:** `tmp/dogfood/10-RUNTIME-FIXES.md` (6 batches)
- **Next step:** Do another `roko plan run` to see if the P0 fixes hold and find the next layer of issues.

### Thread 2: Dead code / built-but-not-wired
- **Done:** 8 modules removed from roko-learn (4,808 LOC). See `tmp/backlog/removed-learn-modules.md`.
- **Still open:**
  - `CognitiveWorkspace` -- types in roko-core + roko-compose, builder exists, event log persistence exists, orchestrate.rs never calls any of it
  - `ExtensionChain` -- hooks called at 5 points but chain always empty (no extension loader)
  - `prd_prompt.rs:152-154` -- hardcoded bardo paths in live agent prompts

### Thread 3: orchestrate.rs decomposition / Runner v2
- **Where:** `tmp/dogfood/07-orchestrate-analysis.md` (v1 decomposition), `tmp/unified/22-PLAN-RUNNER-V2.md` (v2 spec)
- **Status:** 21K-line god object (250 methods). v2 spec written (~2,400-line event-driven replacement).
- **Impact:** Every dogfood fix touches this file. Decomposition or rewrite would unblock parallel work.

### Thread 4: Unified spec migration
- **Where:** `tmp/unified-migration/` (checklist), `tmp/unified-migration-runner/` (automation)
- **Status:** Roadmap written, runner infrastructure built, not yet executing.
- **What:** Rename Engram→Signal, wire 9 protocols, add Graph engine.

### Thread 5: Demo (May 6 a16z)
- **Where:** `tmp/dogfood/09-MAY6-DEMO-BUILD.md`
- **What:** `nunchi` CLI wrapper, `nunchi agents list/audit/resume/replay`, pre-warmed LLM cache, backup tiers.
- **Deadline:** May 6, 2026.

### Thread 6: Pitch materials (May 1)
- **Where:** `tmp/dogfood/12-DECK-AND-MEMO.md`, `11-LANDING-PAGE-UPDATES.md`
- **What:** 13-slide deck, 2,000-word memo, landing page cleanup.
- **Deadline:** May 1, 2026.

### Thread 7: Memory leak
- **Where:** Referenced in `06-run2-deep-findings.md` (F5), `07-consolidated-open-issues.md` (H1)
- **Status:** Open. 9.5-11.5GB RSS after ~17 minutes. Needs DHAT or similar profiling.
- **Impact:** Blocks long-running dogfood sessions.

---

## Key patterns to know

1. **"Built but never wired"** — The codebase has many things implemented but not called. AgentOutput events existed but were never emitted. Always check if something is actually called, not just defined.

2. **Two event systems** — `ServerEvent` (for HTTP SSE) and `DashboardEvent` (for TUI). Overlap but lossy conversion between them (see D5 in `08-statehub-tui-audit.md`).

3. **Plans dir ambiguity** — Plans can be in `plans/` (top-level) or `.roko/plans/` (roko data dir). Several bugs came from code only checking one path.

4. **Batch, not streaming** — Roko waits for agent process exit, reads all output at once. Mori parsed output line-by-line with `--output-format stream-json`. This is why the TUI is blind during runs.

5. **lib.rs linter conflict** — An IDE hook keeps reverting module declaration removals in `crates/roko-learn/src/lib.rs`. The source files for 8 modules are deleted from disk. If compile fails with "file not found" for adversarial, bandit_research, causal, contextual_bandit, kalman, resonant_patterns, shapley, or signal_metabolism — remove those `pub mod` lines from lib.rs.

---

## Key code locations

| What | Path | Notes |
|------|------|-------|
| CLI entry point | `crates/roko-cli/src/main.rs` | |
| Orchestrator (21K lines) | `crates/roko-cli/src/orchestrate.rs` | `PlanRunner` struct, the god object |
| State machine | `crates/roko-orchestrator/src/executor/state_machine.rs` | Phase transitions |
| Agent dispatcher | `crates/roko-agent/src/dispatcher/mod.rs` | Batch dispatch (not streaming) |
| Cascade router | `crates/roko-learn/src/cascade_router.rs` | 3-stage model selection |
| Episode logger | `crates/roko-learn/src/episode_logger.rs` | |
| Efficiency writer | `crates/roko-learn/src/runtime_feedback.rs` | |
| Task parser | `crates/roko-cli/src/task_parser.rs` | `extract_toml_payload()` |
| TUI app | `crates/roko-cli/src/tui/app.rs` | ratatui, `--approval` mode |
| Dashboard events | `crates/roko-core/src/dashboard_snapshot.rs` | `DashboardEvent`, `TaskState` |
| HTTP server | `crates/roko-serve/src/routes/` | ~85 routes on :6677 |
| Process supervisor | `crates/roko-runtime/src/process.rs` | |
| Trustworthy audit (archived) | `tmp/archive/roko-trustworthy/AUDIT.md` | 20 findings, most addressed |
| Removed code notes | `tmp/backlog/removed-learn-modules.md` | 8 modules, reintegration guide |

## How to test

```bash
cargo build --workspace
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace

# Dogfood run
cargo run -p roko-cli -- plan run .roko/plans/unified-migration-phase0 --approval
```
