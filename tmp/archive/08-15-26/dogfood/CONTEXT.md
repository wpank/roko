# Dogfood Context

> **Last updated**: 2026-08-13
>
> **What is this?** This document is the "state of the world" for someone picking up
> dogfood-related work with zero context. Read this first, then dig into the files it
> references.
>
> "Dogfooding" means roko developing itself: running `roko plan run` against real
> implementation plans, recording what breaks, fixing it, and iterating. This folder
> is the QA log from that process.

## Current status (2026-08-13)

- **39 of 43 dogfood findings are RESOLVED.** All P0/P1/P2 issues are fixed.
- **4 items remain OPEN** -- 3 P3 polish + 1 runner v2 spec alignment (Phase E).
- **orchestrate.rs has been DELETED.** Runner v2 (`runner/event_loop.rs`, ~19,846 lines)
  is now the sole plan execution engine. All orchestrate.rs references in this folder are historical.
- **event_loop.rs (~19,846 lines) is the current god-object concern.** It absorbed
  orchestrate.rs's functionality and has the same decomposition problem.
- **Engram-to-Signal rename: DONE (2026-08-12).** `pub type Signal = Engram` alias landed in
  `crates/roko-core/src/engram.rs` with a `signal.rs` re-export module. The underlying struct
  is still named `Engram` and `engrams.jsonl` is still the file name on disk (~29 files), but
  new code can use `Signal` everywhere. Full struct rename deferred to Phase 1 (Cell trait).
- **contextual_bandit.rs came back as dead code.** Removed in April 2026 (1,372 LOC) but
  re-added by a batch agent run. Lives in `roko-learn/src/contextual_bandit.rs`, only
  referenced from one test. No production callers.
- **All 6 critical dogfood fixes from 2026-04-26 are RESOLVED** (force_shutdown self-kill,
  executor.json persistence, efficiency event flush, model fallback, implementation dispatch,
  test compilation).
- **The May 6 a16z demo** (files 09, 11, 12) is historical -- that date has passed.
- **Runner v2 streaming, TUI, and persistence all work.** The core dogfood workflow
  (`prd idea -> prd draft -> prd plan -> plan run -> dashboard`) is fully operational.

---

## What is Roko

Roko is a Rust toolkit (18 crates, ~177K LOC) for building agents that build themselves.
It reads PRDs, generates implementation plans, executes tasks via Claude/Codex agents,
validates results through gate pipelines, and persists everything.

Key entry points:
- **CLI main**: `crates/roko-cli/src/main.rs`
- **Plan runner (current)**: `crates/roko-cli/src/runner/event_loop.rs` (~19,846 lines)
- **Runner module**: `crates/roko-cli/src/runner/` (25 files)
- **Agent dispatcher**: `crates/roko-agent/src/dispatcher/mod.rs`

The self-hosting workflow:
```
roko prd idea "..." -> roko prd draft -> roko prd plan <slug> -> roko plan run plans/ -> roko dashboard
```

---

## What happened across sessions

### Session 1 (2026-04-26, earlier in the day)

Killed a hung roko process (11.5GB RAM). Audited dogfood findings from 3 real plan-runner
executions. Fixed 6 critical issues (force_shutdown self-kill, no executor.json persistence,
efficiency events not flushed, model fallback to haiku, implementation phase never dispatching,
test compilation). Created consolidated open issues doc. Updated checklist to 21/56 done.

### Session 2 (2026-04-26, later)

#### 1. Audited the roko-trustworthy runner

The `roko-trustworthy` runner was a 24-batch overnight Codex run that added trustworthiness
infrastructure. All 24 batches passed. Found 6 gaps:

| Gap | Status |
|-----|--------|
| `ContextualBanditPolicy` dead code | RESOLVED -- removed 1,372 LOC |
| `CognitiveWorkspace` not wired | OPEN -- types exist but runner never produces one |
| `ExtensionChain` always empty | OPEN -- hooks called but chain has no extensions |
| Warm-agent pooling absent | OPEN -- `reuse_policy_id` field exists but no process reuse |
| `prd_prompt.rs` bardo paths | OPEN -- hardcoded bardo paths in live agent prompts |
| E2E test is `#[ignore]` | OPEN -- needs mock fixture |

#### 2. Removed 4,808 lines of dead code from roko-learn

8 modules with zero production callers were removed: `contextual_bandit.rs` (1,372 LOC),
`bandit_research.rs` (862), `causal.rs` (699), `shapley.rs` (518), `resonant_patterns.rs` (373),
`kalman.rs` (354), `adversarial.rs` (321), `signal_metabolism.rs` (309). Recoverable from git history.
Reintegration notes at `tmp/backlog/removed-learn-modules.md`.

### Post-April 2026 changes

- **orchestrate.rs was DELETED.** Runner v2 became the sole execution path (Phase D complete).
- **Runner v2 was made the default** for all `plan run` invocations, not just `--approval` mode.
- **event_loop.rs grew to ~19,846 lines**, absorbing orchestrate.rs functionality. This is the
  new god-object concern.
- **All P1/P2 items were resolved** through runner v2 and subsequent fix batches.
- **Engram-to-Signal rename landed (2026-08-12).** `pub type Signal = Engram` alias + `signal.rs`
  re-export module. The underlying struct is still `Engram` and `engrams.jsonl` is still on disk,
  but new code can import `Signal`. Full struct rename deferred to Phase 1.
- **contextual_bandit.rs re-appeared.** Was removed in April 2026 (1,372 LOC dead code) but a
  batch agent run re-added it. Still dead code -- only referenced from one test, no prod callers.

---

## The dogfood folder (tmp/dogfood/)

This folder is the QA log from dogfooding Roko -- actually running `roko plan run` and
recording what breaks. Comes from 3 real executions on 2026-04-26.

### Read in this order

| # | File | What | Status |
|---|------|------|--------|
| 1 | **00-INDEX.md** | Master checklist (43 items, 39 done, 4 open). Start here. | CURRENT |
| 2 | **CONTEXT.md** | This file -- state of the world for new sessions | CURRENT |
| 3 | **STATE-OF-THE-WORLD.md** | Comprehensive project state doc (written 2026-04-26, updated 2026-08-13) | CORRECTED (see note below) |
| 4 | **09-MAY6-DEMO-BUILD.md** | Demo spec for May 6 a16z pitch | HISTORICAL (date passed) |
| 5 | **12-DECK-AND-MEMO.md** | 13-slide deck + memo spec | HISTORICAL (date passed) |
| 6 | **11-LANDING-PAGE-UPDATES.md** | nunchi.network cleanup | HISTORICAL (date passed) |
| 7 | **archive/** | Historical run logs, superseded consolidations | REFERENCE ONLY |

**STATE-OF-THE-WORLD.md note**: Originally written 2026-04-26, updated 2026-08-13 with
corrections. Key updates: orchestrate.rs marked as DELETED, event_loop.rs line counts
corrected, all P0/P1/P2 marked RESOLVED, Engram-to-Signal status updated, demo sections
marked HISTORICAL. Useful for understanding the architectural journey and design rationale.

---

## Open threads (priority order)

### Thread 1: event_loop.rs decomposition

- **Where**: `crates/roko-cli/src/runner/event_loop.rs` (~19,846 lines)
- **Status**: OPEN. This file replaced orchestrate.rs but has grown to similar size.
- **Impact**: Same problems as orchestrate.rs -- hard to work on in parallel, merge conflicts,
  too many responsibilities in one file.
- **Next step**: Plan a decomposition similar to what was done for orchestrate.rs.

### Thread 2: Runner v2 missing persistence

- **Where**: `00-INDEX.md` items S4, S7
- **Status**: OPEN. Runner v2 does not write to `cascade-router.json`, `gate-thresholds.json`,
  or fire replan-on-gate-failure. These were orchestrate.rs features that didn't survive the transition.
- **Impact**: Cascade router and gate threshold learning are degraded (they don't persist between runs).

### Thread 3: Engram-to-Signal rename

- **Where**: `crates/roko-core/src/engram.rs`, ~29 files referencing `engrams.jsonl`
- **Status**: PARTIALLY DONE (2026-08-12). `pub type Signal = Engram` alias landed. New code
  can use `Signal` everywhere. The underlying struct is still `Engram` and `engrams.jsonl` is
  still the file name on disk. Full struct rename and file-path rename deferred to Phase 1.
- **Impact**: Terminology gap narrowed -- new code uses `Signal`, old code still says `Engram`.

### Thread 4: Dead code / built-but-not-wired

- **Done**: 8 modules removed from roko-learn (4,808 LOC) in April 2026.
- **Regressed**: `contextual_bandit.rs` (1,372 LOC) was re-added by a batch agent run. It
  exists in `roko-learn/src/contextual_bandit.rs` and is only referenced from one test
  (`phase0_wiring.rs`). No production callers. Should be removed again.
- **Still open**:
  - `CognitiveWorkspace` -- types in roko-core + roko-compose, builder exists, runner never produces one
  - `ExtensionChain` -- hooks called at 5 points but chain always empty (no extension loader)
  - `prd_prompt.rs` -- hardcoded bardo paths in live agent prompts

---

## Key patterns to know

1. **"Built but never wired"** -- The codebase has many things implemented but not called. AgentOutput events existed but were never emitted. Always check if something is actually called, not just defined.

2. **Two event systems** -- `ServerEvent` (for HTTP SSE) and `DashboardEvent` (for TUI). Overlap but lossy conversion between them.

3. **Plans dir ambiguity** -- Plans can be in `plans/` (top-level) or `.roko/plans/` (roko data dir). Several bugs came from code only checking one path.

4. **Streaming is now wired** -- Runner v2 uses `--output-format stream-json` and parses output line-by-line. This fixed the "TUI is blind" problem that dominated early dogfood findings.

5. **God-object migration** -- orchestrate.rs (21K lines) was replaced by event_loop.rs, which is now ~19,800 lines. The pattern of "one file absorbs everything" keeps recurring.

---

## Key code locations

| What | Path | Notes |
|------|------|-------|
| CLI entry point | `crates/roko-cli/src/main.rs` | |
| Plan runner (current) | `crates/roko-cli/src/runner/event_loop.rs` | ~19,846 lines, the current god-object |
| Runner module | `crates/roko-cli/src/runner/` | 25 files |
| State machine | `crates/roko-orchestrator/src/executor/state_machine.rs` | Phase transitions |
| Agent dispatcher | `crates/roko-agent/src/dispatcher/mod.rs` | Now streams (not batch) |
| Cascade router | `crates/roko-learn/src/cascade_router.rs` | 3-stage model selection |
| Episode logger | `crates/roko-learn/src/episode_logger.rs` | |
| Efficiency writer | `crates/roko-learn/src/runtime_feedback.rs` | |
| Task parser | `crates/roko-cli/src/task_parser.rs` | `extract_toml_payload()` |
| TUI app | `crates/roko-cli/src/tui/app.rs` | ratatui, `--approval` mode |
| Dashboard events | `crates/roko-core/src/dashboard_snapshot.rs` | `DashboardEvent`, `TaskState` |
| HTTP server | `crates/roko-serve/src/routes/` | ~85 routes on :6677 |
| Process supervisor | `crates/roko-runtime/src/process.rs` | |
| Engram/Signal type | `crates/roko-core/src/engram.rs` | `pub type Signal = Engram` alias landed 2026-08-12; struct still named `Engram` |
| Signal re-exports | `crates/roko-core/src/signal.rs` | Convenience re-export module for the Signal alias |

## How to test

```bash
cargo build --workspace
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace

# Dogfood run
cargo run -p roko-cli -- plan run .roko/plans/ --approval
```
