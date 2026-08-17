# Dogfood Findings Index

> **Last updated**: 2026-08-13
>
> **What is this?** "Dogfooding" means using roko to develop itself. Roko is a Rust
> agent runtime (18 crates, ~177K LOC) that reads PRDs, generates implementation plans,
> executes tasks via LLM agents, validates results through gate pipelines, and persists
> everything. This folder documents what broke when we actually ran `roko plan run` against
> real plans and recorded every failure. Three real executions were done on 2026-04-26.
> The findings drove 38 fixes and a full rewrite of the plan runner.

> **STATUS: HISTORICAL -- 39/43 findings resolved (2026-08-13)**
>
> This dogfood session (2026-04-26) has been substantially resolved.
> The 4 remaining items (P3 or spec-alignment) are tracked in `.roko/GAPS.md`.
> This file is preserved for historical reference.

## Current status (2026-08-13)

- **39 of 43 items RESOLVED.** All P0, P1, and P2 issues are fixed.
- **4 items remain OPEN** -- 3 P3 (polish/tech debt) + 1 runner v2 spec alignment (Phase E).
- **orchestrate.rs has been DELETED.** Runner v2 (`crates/roko-cli/src/runner/event_loop.rs`)
  replaced it entirely. All references to orchestrate.rs in this folder are historical.
- **event_loop.rs is now ~19,846 lines** and is the current god-object concern (same problem
  orchestrate.rs had, reincarnated).
- **Engram-to-Signal rename: DONE (2026-08-12).** `pub type Signal = Engram` alias landed in
  `crates/roko-core/src/engram.rs` with a `signal.rs` re-export module. The underlying struct
  is still named `Engram` and `engrams.jsonl` is still the file name on disk (~29 files), but
  new code can use `Signal` everywhere. Full struct rename deferred to Phase 1 (Cell trait).
- **contextual_bandit.rs came back as dead code.** It was removed (1,372 LOC) in April 2026
  but was re-added by a batch agent run. It exists in `roko-learn/src/contextual_bandit.rs`
  (1,372 lines) and is only referenced from one test (`phase0_wiring.rs`). No production callers.
- **All 6 critical dogfood fixes from 2026-04-26 are RESOLVED** (force_shutdown self-kill,
  executor.json persistence, efficiency event flush, model fallback, implementation dispatch,
  test compilation).
- The May 6 a16z demo items (09, 11, 12) are **historical** -- that date has passed.

---

## How to use this file

This is both an index and a checklist. Each issue has a status checkbox.
When fixing an issue, update the checkbox to `[x]`, add a brief note of what was done,
and move the resolution details to `archive/`.

### Editing instructions

1. **New finding**: Create a numbered file (`NN-description.md`), add it to the Files
   table below, and add checklist entries for each sub-issue.
2. **Resolving an issue**: Check the box `[x]`, add `(branch: wp-xxx)` note, write
   details in `archive/resolved-YYYY-MM-DD.md`.
3. **Updating status**: Just edit the checkbox and add a parenthetical note.
4. **Cross-references**: Use `[descriptive text](filename.md)` links.
5. **Priority labels**: `P0` = blocks dogfooding, `P1` = degrades experience,
   `P2` = missing feature, `P3` = polish.

---

## Files

### Active

| # | File | Description | Status |
|---|---|---|---|
| 09 | [09-MAY6-DEMO-BUILD.md](09-MAY6-DEMO-BUILD.md) | May 6 a16z demo -- CLI commands, cached LLM proxy, backup tiers | HISTORICAL (date passed) |
| 11 | [11-LANDING-PAGE-UPDATES.md](11-LANDING-PAGE-UPDATES.md) | Landing page alignment -- remove mock data, add /changelog, update positioning | HISTORICAL (date passed) |
| 12 | [12-DECK-AND-MEMO.md](12-DECK-AND-MEMO.md) | Deck (13 slides) + pre-read memo (2,000 words) build checklist | HISTORICAL (date passed) |

### Context / Onboarding

| File | Description | Status |
|---|---|---|
| [CONTEXT.md](CONTEXT.md) | State of the world for new sessions -- read this first | Updated 2026-08-13 |
| [STATE-OF-THE-WORLD.md](STATE-OF-THE-WORLD.md) | Comprehensive project state doc from 2026-04-26 | STALE -- runner v2 is now default, orchestrate.rs deleted |

### Archived (historical run logs, superseded consolidations)

| File | Description |
|---|---|
| [archive/01-endpoint-audit.md](archive/01-endpoint-audit.md) | HTTP endpoint audit -- all issues resolved or tracked here |
| [archive/02-plan-runner-gaps.md](archive/02-plan-runner-gaps.md) | Plan runner bugs -- all consolidated here |
| [archive/03-resource-management.md](archive/03-resource-management.md) | OOM from zombie processes -- fixed |
| [archive/04-run2-observations.md](archive/04-run2-observations.md) | Run 2 observations -- consolidated here |
| [archive/05-mori-vs-roko-agent-wiring.md](archive/05-mori-vs-roko-agent-wiring.md) | Deep mori-to-roko comparison (good reference for root causes) |
| [archive/06-run2-deep-findings.md](archive/06-run2-deep-findings.md) | Run 2 deep findings -- F1-F9 tracked here |
| [archive/07-consolidated-open-issues.md](archive/07-consolidated-open-issues.md) | Intermediate consolidation -- superseded by this file |
| [archive/07-orchestrate-analysis.md](archive/07-orchestrate-analysis.md) | orchestrate.rs decomposition plan -- MOOT (orchestrate.rs deleted, runner v2 replaced it) |
| [archive/08-statehub-tui-audit.md](archive/08-statehub-tui-audit.md) | StateHub-to-TUI audit -- fixes done or tracked here |
| [archive/10-RUNTIME-FIXES.md](archive/10-RUNTIME-FIXES.md) | 6 fix batches -- mostly complete, remainder tracked here |
| [archive/13-SESSION-CONTEXT-2026-04-26.md](archive/13-SESSION-CONTEXT-2026-04-26.md) | Session retrospective from April 26 |
| [archive/resolved-2026-04-26.md](archive/resolved-2026-04-26.md) | Original resolved issues record |

---

## Master Checklist

### Fixes Applied (2026-04-26, branch: wp-arch2) -- ALL RESOLVED

- [x] **#1** TUI invisible to plan runner -- `--approval` shares StateHub in-process
- [x] **#3** Enrichment too aggressive -- `skip_enrichment = true` in `[meta]`
- [x] **#7** StateHub not exposed via HTTP -- `GET /api/statehub/snapshot`
- [x] **#10** No health endpoint -- `GET /health` (top-level, no auth)
- [x] **#14** Config v1 warnings spam -- `std::sync::Once` in `from_toml()`
- [x] **TUI-1** TUI crash on ws_client (no tokio runtime) -- `Handle::try_current()` guard
- [x] **TUI-2** Ctrl+C leaves zombie processes -- `libc::kill(0, SIGTERM)` + 3s grace
- [x] **F1** plans_dir resolution bug -- `ensure_task_tracker` + `dispatch_agent_with` check `.roko/plans/` fallback
- [x] **F3** AgentOutput never emitted -- `emit_server_event(ServerEvent::AgentOutput)` in dispatch
- [x] **F4** TaskState lacks title -- added `title: String` to TaskState + TaskStarted event
- [x] **C5** force_shutdown() kills self via `kill(0, SIGTERM)` -- mask SIGTERM before group signal, restore after
- [x] **#2** No executor.json written during run -- `save_state()` after every phase transition in `apply_event_and_emit()`
- [x] **F2** Model routing falls back to haiku -- merge configured models into candidates, fix hardcoded sonnet fallback
- [x] **F6** Implementation phase never dispatches -- added `ensure_task_tracker()` call at start of `handle_implementing()`
- [x] **#5** Episodes not written during run -- EpisodeLogger already flushes per-write; root cause was F6
- [x] **#6** Efficiency events not tracked -- added `flush()` to `append_efficiency_event()`

### P0 -- Blocks Dogfooding -- ALL RESOLVED

- [x] **#2** No executor.json written during run (2026-04-26, branch: wp-arch2)
- [x] **F2** Model routing falls back to haiku (2026-04-26, branch: wp-arch2)
- [x] **F6** Implementation phase never dispatches (2026-04-26, branch: wp-arch2)

### P1 -- Degrades Experience -- ALL RESOLVED

- [x] **#5** Episodes not written during run -- root cause was F6 (implementation never dispatching)
- [x] **#6** Efficiency events not tracked -- added `flush()` to `append_efficiency_event()`
- [x] **#8** TOML parse fails on markdown-fenced LLM output -- `extract_toml_payload()` + `TasksFile::parse_agent_output()` strips fences
- [x] **F9** TUI log bar garbled -- TUI mode redirects all tracing to `.roko/roko.log`
- [x] **M3** Tokens/cost show "0k/$0.00" in TUI -- `emit_efficiency_event()` now publishes token/cost DashboardEvents
- [x] **#9** Enrichment timeouts too short (120s) -- fixed by runner v2: uses `RunConfig::timeout_secs`
- [x] **M1** No streaming agent output -- fixed by runner v2: full `--output-format stream-json` parsing in `runner/agent_stream.rs`
- [x] **M2** Model shows "-" in TUI agent roster -- `tui_bridge.agent_spawned()` now forwards model name
- [x] **F5** Memory leak -- 9.5GB RSS after 17 minutes -- fixed by runner v2: streaming, per-task flushing, lightweight RunState

### P2 -- Missing Features -- ALL RESOLVED

- [x] **#4** Codex backend -- `CodexAgent` in `codex_agent.rs` (979 lines), wired via provider system
- [x] **#11** Plan detail routes -- 12 routes in `routes/plans.rs`
- [x] **#13** Executor state endpoint -- `GET /api/executor/state`
- [x] **#17** Learn/router endpoint -- `GET /api/learn/router`
- [x] **#12** Knowledge endpoint -- `GET /api/knowledge?q=<topic>` alias in `routes/neuro.rs`

### P3 -- Polish / Tech Debt -- 3 OPEN

- [x] **#16** Worktree isolation -- `WorktreeManager` wired, `executor.use_worktrees` config field (off by default)
- [x] **S5** TUI log -- real structured tracing to `.roko/tui.log`

- [ ] **#15** Enrichment artifacts mostly empty/minimal -- moot with skip_enrichment (OPEN, low priority)
- [ ] **S4** signals.jsonl stays at 0 lines -- conductor signals write to `engrams.jsonl` instead; `signals.jsonl` path in `layout.rs` is dead. The `Signal` type alias now exists (`pub type Signal = Engram`) but `engrams.jsonl` is still the file name on disk. Full file-path rename deferred to Phase 1. (OPEN)
- [ ] **S7** learn/ files stale -- runner v2 (event_loop.rs) only writes efficiency + episodes; cascade-router.json and gate-thresholds.json are not updated by runner v2. (OPEN)

### Rewrite: Plan Runner v2

**Background**: The original orchestrator was `orchestrate.rs` (21K lines, a god object with
250+ methods). Runner v2 replaced it with an event-driven architecture in `runner/`.

**Current state (2026-08-13)**: orchestrate.rs has been **DELETED**. Runner v2 in
`crates/roko-cli/src/runner/event_loop.rs` (~19,846 lines) is the sole plan execution engine.
It has grown to exhibit the same god-object pattern that orchestrate.rs had.

- [x] Phase A: Build `runner/` module alongside orchestrate.rs -- 10 files, 2,181 lines (2026-04)
- [x] Phase B: Wire into CLI -- active for `--approval` mode (2026-04)
- [x] Phase C: Make runner v2 the default for all `plan run` invocations (2026-05)
- [x] Phase D: Delete orchestrate.rs (DONE -- file no longer exists on disk)
- [ ] Phase E: Align with unified spec (type renames, Activity recording) (OPEN)

**Known runner v2 gaps** (inherited from the transition):
- Does NOT update `cascade-router.json` (no CascadeRouter persistence in runner v2)
- Does NOT update `gate-thresholds.json` (no AdaptiveThresholds persistence in runner v2)
- Does NOT fire replan-on-gate-failure

**New concern**: event_loop.rs at ~19,800 lines needs decomposition. It has absorbed
functionality from orchestrate.rs and grown beyond reasonable file size. This is the same
god-object problem that motivated the runner v2 rewrite in the first place.

### Legacy Refactor: orchestrate.rs Decomposition (MOOT)

Detailed plan archived in [archive/07-orchestrate-analysis.md](archive/07-orchestrate-analysis.md).
This is entirely historical -- orchestrate.rs was deleted, not decomposed. The decomposition
plan is only useful as a reference for what functionality event_loop.rs now needs to manage.

---

## Quick Stats

| Category | Total | Done | Open |
|----------|-------|------|------|
| Fixes applied (2026-04-26) | 16 | 16 | 0 |
| P0 (blocks dogfooding) | 3 | 3 | 0 |
| P1 (degrades experience) | 9 | 9 | 0 |
| P2 (missing features) | 5 | 5 | 0 |
| P3 (polish) | 5 | 2 | 3 |
| Runner v2 phases | 5 | 4 | 1 |
| **Total** | **43** | **39** | **4** |

> **Remaining open items**: `#15` (enrichment artifacts), `S4` (signals.jsonl dead path),
> `S7` (learn/ files stale), `Phase E` (runner v2 spec alignment). All low priority.
