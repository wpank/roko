# Dogfood Findings — 2026-04-26

> **How to use this file**: This is both an index and a checklist. Each issue has a
> status checkbox. When fixing an issue, update the checkbox to `[x]`, add a brief
> note of what was done, and move the resolution details to `archive/`.

## Editing Instructions

When making changes to dogfood docs:

1. **New finding**: Create a numbered file (`NN-description.md`), add it to the Files
   table below, and add checklist entries for each sub-issue.
2. **Resolving an issue**: Check the box `[x]`, add `(branch: wp-xxx)` note, write
   details in `archive/resolved-YYYY-MM-DD.md`.
3. **Updating status**: Just edit the checkbox and add a parenthetical note.
4. **Cross-references**: Use `[descriptive text](filename.md)` links.
5. **Priority labels**: `P0` = blocks dogfooding, `P1` = degrades experience,
   `P2` = missing feature, `P3` = polish.

## Files

### Active

| # | File | Description |
|---|---|---|
| 09 | [09-MAY6-DEMO-BUILD.md](09-MAY6-DEMO-BUILD.md) | **May 6 a16z demo** — CLI commands, cached LLM proxy, backup tiers |
| 11 | [11-LANDING-PAGE-UPDATES.md](11-LANDING-PAGE-UPDATES.md) | Landing page alignment — remove mock data, add /changelog, update positioning |
| 12 | [12-DECK-AND-MEMO.md](12-DECK-AND-MEMO.md) | Deck (13 slides) + pre-read memo (2,000 words) build checklist |

### Context / Onboarding

| File | Description | Notes |
|---|---|---|
| [CONTEXT.md](CONTEXT.md) | State of the world for new sessions | Slightly stale: runner v2 IS wired for `--approval` mode now |
| [STATE-OF-THE-WORLD.md](STATE-OF-THE-WORLD.md) | Comprehensive project state doc | Slightly stale: runner v2 IS wired for `--approval` mode now |

### Archived (historical run logs, superseded consolidations)

| File | Description |
|---|---|
| [archive/01-endpoint-audit.md](archive/01-endpoint-audit.md) | HTTP endpoint audit — all issues resolved or tracked here |
| [archive/02-plan-runner-gaps.md](archive/02-plan-runner-gaps.md) | Plan runner bugs — all consolidated here |
| [archive/03-resource-management.md](archive/03-resource-management.md) | OOM from zombie processes — fixed |
| [archive/04-run2-observations.md](archive/04-run2-observations.md) | Run 2 observations — consolidated here |
| [archive/05-mori-vs-roko-agent-wiring.md](archive/05-mori-vs-roko-agent-wiring.md) | Deep mori→roko comparison (good reference for root causes) |
| [archive/06-run2-deep-findings.md](archive/06-run2-deep-findings.md) | Run 2 deep findings — F1-F9 tracked here |
| [archive/07-consolidated-open-issues.md](archive/07-consolidated-open-issues.md) | Intermediate consolidation — superseded by this file |
| [archive/07-orchestrate-analysis.md](archive/07-orchestrate-analysis.md) | orchestrate.rs decomposition plan — superseded by runner v2 |
| [archive/08-statehub-tui-audit.md](archive/08-statehub-tui-audit.md) | StateHub → TUI audit — fixes done or tracked here |
| [archive/10-RUNTIME-FIXES.md](archive/10-RUNTIME-FIXES.md) | 6 fix batches — mostly complete, remainder tracked here |
| [archive/13-SESSION-CONTEXT-2026-04-26.md](archive/13-SESSION-CONTEXT-2026-04-26.md) | Session retrospective from April 26 |
| [archive/resolved-2026-04-26.md](archive/resolved-2026-04-26.md) | Original resolved issues record |

---

## Master Checklist

### Fixes Applied (branch: wp-arch2)

- [x] **#1** TUI invisible to plan runner — `--approval` shares StateHub in-process
- [x] **#3** Enrichment too aggressive — `skip_enrichment = true` in `[meta]`
- [x] **#7** StateHub not exposed via HTTP — `GET /api/statehub/snapshot`
- [x] **#10** No health endpoint — `GET /health` (top-level, no auth)
- [x] **#14** Config v1 warnings spam — `std::sync::Once` in `from_toml()`
- [x] **TUI-1** TUI crash on ws_client (no tokio runtime) — `Handle::try_current()` guard
- [x] **TUI-2** Ctrl+C leaves zombie processes — `libc::kill(0, SIGTERM)` + 3s grace
- [x] **F1** plans_dir resolution bug — `ensure_task_tracker` + `dispatch_agent_with` check `.roko/plans/` fallback
- [x] **F3** AgentOutput never emitted — `emit_server_event(ServerEvent::AgentOutput)` in dispatch
- [x] **F4** TaskState lacks title — added `title: String` to TaskState + TaskStarted event
- [x] **C5** force_shutdown() kills self via `kill(0, SIGTERM)` — mask SIGTERM before group signal, restore after

### P0 — Blocks Dogfooding

- [x] **#2** No executor.json written during run — `save_state()` after every phase transition in `apply_event_and_emit()` (branch: wp-arch2)
- [x] **F2** Model routing falls back to haiku — merge configured models into candidates, fix hardcoded sonnet fallback, pass candidates for force_backend (branch: wp-arch2)
- [x] **F6** Implementation phase never dispatches — added `ensure_task_tracker()` call at start of `handle_implementing()` (branch: wp-arch2)

### P1 — Degrades Experience

- [x] **#5** Episodes not written during run — EpisodeLogger already flushes per-write; issue was implementation never dispatching (F6)
- [x] **#6** Efficiency events not tracked — added `flush()` to `append_efficiency_event()` + added efficiency.jsonl to `flush_logs()` (branch: wp-arch2)
- [x] **#8** TOML parse fails on markdown-fenced LLM output — `extract_toml_payload()` + `TasksFile::parse_agent_output()` in `task_parser.rs` strips fences before parsing (verified: passing test `parse_agent_output_strips_fences`)
- [x] **F9** TUI log bar garbled — TUI mode redirects all tracing to `.roko/roko.log` file instead of stderr (`main.rs:1576-1628`)
- [x] **M3** Tokens/cost show "0k/$0.00" in TUI — `emit_efficiency_event()` now publishes `input_tokens`, `output_tokens`, `cost_usd` DashboardEvents after every dispatch; runner v2 also accumulates live from stream

- [x] **#9** Enrichment timeouts too short (120s) — **fixed by runner v2 default**: legacy 120s path no longer used; runner v2 uses `RunConfig::timeout_secs` from roko.toml. Plan authors should set `timeout_secs` in tasks.toml for task-level overrides.

- [x] **M1** No streaming agent output — **fixed by runner v2 default**: full `--output-format stream-json` parsing in `runner/agent_stream.rs`. Runner v2 is now the default for ALL `plan run` invocations (not just `--approval`).

- [x] **M2** Model shows "-" in TUI agent roster — **fixed**: `tui_bridge.agent_spawned()` now accepts and forwards the model name. `event_loop.rs` passes the resolved model (from task `model_hint` or config default) to the TUI event.

- [x] **F5** Memory leak — 9.5GB RSS after 17 minutes — **fixed by runner v2 default**: runner v2 uses streaming (no buffered agent output), per-task state flushing, and lightweight `RunState` instead of the 21K-line PlanRunner with unbounded vectors.

### P2 — Missing Features

- [x] **#4** Codex backend — `CodexAgent` in `codex_agent.rs` (979 lines), wired via provider system (`"codex"` → `ProviderKind::OpenAiCompat`); full JSON-RPC-over-stdio (Codex CLI) still TODO
- [x] **#11** Plan detail routes — 12 routes in `routes/plans.rs`: `GET /plans/{id}`, `/plans/{id}/tasks`, `/plans/{id}/status`, `/plans/{id}/gates`, `/plans/{id}/reviews`, etc.
- [x] **#13** Executor state endpoint — `GET /api/executor/state` in `learning/mod.rs`
- [x] **#17** Learn/router endpoint — `GET /api/learn/router` in `learning/mod.rs` (also `/learning/cascade-router`)

- [x] **#12** Knowledge endpoint — **fixed**: `GET /api/knowledge?q=<topic>` alias added in `routes/neuro.rs`, proxies to the same knowledge store as `POST /api/neuro/query`.

### P3 — Polish / Tech Debt

- [x] **#16** Worktree isolation — `WorktreeManager` wired in `PlanRunner`, `executor.use_worktrees` config field (off by default)
- [x] **S5** TUI log — real structured tracing to `.roko/tui.log` via `tui_log_dispatch()` in `tui/app.rs`

- [ ] **#15** Enrichment artifacts mostly empty/minimal — moot with skip_enrichment
- [ ] **S4** signals.jsonl stays at 0 lines — conductor signals write to `engrams.jsonl` instead; `signals.jsonl` path in `layout.rs` is dead
- [ ] **S7** learn/ files stale — orchestrate.rs path writes efficiency/cascade-router/gate-thresholds; runner v2 only writes efficiency + episodes (cascade-router and gate-thresholds not updated by runner v2)

### Rewrite: Plan Runner v2

**Spec**: [`tmp/unified/22-PLAN-RUNNER-V2.md`](../unified/22-PLAN-RUNNER-V2.md)

Replaces orchestrate.rs (21K lines) with ~2,400-line event-driven runner.
Fixes M1 (streaming), F5 (memory), and most P1/P2 items above by design.

- [x] Phase A: Build `runner/` module alongside orchestrate.rs — 10 files, 2,181 lines
- [x] Phase B: Wire into CLI — active for `--approval` mode in `commands/plan.rs:221-319`
- [x] Phase C: Make runner v2 the default for all `plan run` invocations — old PlanRunner path fully replaced
- [ ] Phase D: Deprecate orchestrate.rs → `orchestrate_legacy.rs`
- [ ] Phase E: Align with unified spec (type renames, Activity recording)

**Known runner v2 gaps** (vs orchestrate.rs):
- Does NOT update `cascade-router.json` (no CascadeRouter persistence)
- Does NOT update `gate-thresholds.json` (no AdaptiveThresholds persistence)
- Does NOT fire replan-on-gate-failure

### Legacy Refactor: orchestrate.rs Decomposition (superseded by runner v2)

Detailed plan archived in [archive/07-orchestrate-analysis.md](archive/07-orchestrate-analysis.md).
Only relevant if orchestrate.rs is kept long-term for the non-approval path.

---

## Quick Stats

| Category | Total | Done | Open |
|----------|-------|------|------|
| Fixes applied | 16 | 16 | 0 |
| P0 (blocks dogfooding) | 3 | 3 | 0 |
| P1 (degrades experience) | 9 | 9 | 0 |
| P2 (missing features) | 5 | 5 | 0 |
| P3 (polish) | 5 | 2 | 3 |
| Runner v2 phases | 5 | 3 | 2 |
| **Total** | **43** | **38** | **5** |
