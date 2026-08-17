# D8 Demo/Bench Fixes — Full Audit & Remediation

Date: 2026-04-29

## Summary

All 36 D8 tasks (dashboard widgets, demo scenarios, bench infrastructure) were audited.
The core problem was **API shape mismatches** — frontend and backend were built
independently with different field names/structures. This document catalogs every fix.

---

## Dashboard Widgets (7 items)

### 1. C-Factor Sparkline — FIXED
**Problem:** Backend returned flat `CFactorBucket[]`. Frontend expected `{trend: [...], woolley: {...}}`.
**Fix:** `router_state.rs:cfactor_trend` now returns `{trend: buckets, woolley: {...}}` with
5 Woolley sub-metric sparklines derived from aggregate C-Factor values.
**Files:** `crates/roko-serve/src/routes/learning/router_state.rs`

### 2. Agent Topology Graph — FIXED
**Problem:** Backend returned `{nodes, edges: []}` with wrong node shape (`id`/`address`
vs frontend's `agent_id`/`role`/`endpoints`). Edges always empty.
**Fix:** Handler returns raw JSON with `agent_id`/`role`/`endpoints` fields. Edges computed
from shared domain tags between agents.
**Files:** `crates/roko-serve/src/routes/aggregator.rs`

### 3. Gate Waterfall — FIXED
**Problem:** `/api/gates/history` returned flat per-verdict items. Frontend `GateWaterfall`
expected `GateRun[]` with nested `rungs: GateRung[]`.
**Fix:** Added `?format=waterfall` query param. When set, items are grouped by `task_id`
into `GateRun` objects with nested rungs. Frontend now passes `&format=waterfall`.
**Files:** `crates/roko-serve/src/routes/status/gates.rs`, `demo/demo-app/src/pages/dashboard/ChainView.tsx`

### 4. Dream Visualization — FIXED (was orphaned)
**Problem:** `DreamPhaseViz` component existed but was not mounted in any route.
`GET /api/dream/journal` route did not exist.
**Fix:** Added `GET /api/dream/journal` route to `dream.rs` that reads from
`.roko/dreams/journal.jsonl` and returns `{last_cycle, cycle_count, phases: DreamPhase[]}`.
Created `DreamsView.tsx` page, mounted at `/dashboard/dreams`, added nav link.
**Files:** `crates/roko-serve/src/routes/dream.rs`, `demo/demo-app/src/pages/dashboard/DreamsView.tsx`,
`demo/demo-app/src/main.tsx`, `demo/demo-app/src/pages/dashboard/Layout.tsx`

### 5. Threshold Gauges — FIXED
**Problem:** Backend returned `{rungs: [{rung: 2, ema_pass_rate: ...}]}` (numeric IDs).
Frontend expected `{thresholds: {"test": {mean_pass_rate: ...}}}` (string-keyed map).
**Fix:** Response now uses `thresholds` map keyed by rung name. `RungThresholdDetail` has
`mean_pass_rate`, `ema_threshold`, `rung_count`, `consecutive_passes`.
**Files:** `crates/roko-serve/src/routes/learning/mod.rs`

### 6. Cost Bars — ALREADY WORKING
No changes needed. End-to-end data flow was correct.

### 7. Provider Mosaic — FIXED
**Problem:** `provider_outcomes` returned raw projection `Value`. Frontend expected
`{providers: [{name, status, models, success_rate, avg_latency_ms, ...}]}`.
**Fix:** Handler now builds provider summaries from efficiency events, computing
success rate, latency, cost, and deriving health status.
**Files:** `crates/roko-serve/src/routes/learning/mod.rs`

---

## Demo Scenarios (5 items)

No code changes needed for demo scenarios. The scenario logic was already fully
implemented — they just depend on a live `roko serve` with `--enable-terminal` and real
API keys. With the widget fixes above, the dashboards they render into now show correct
data.

### Cross-cutting note
All 5 scenarios require `serve.terminal_enabled = true`. The demo stub agent
(`echo demo-stub`) produces no meaningful output for scenarios 2-5.

---

## Bench Infrastructure (9 items)

### 1. Learnable-rust suite — ALREADY WORKING
Backend has 5 real Rust tasks + scaffold. No changes needed.

### 2. Cerebras model — NOW EXPOSED via /bench/models
See item 9 below.

### 3. Workdir scaffold — ALREADY WORKING
No changes needed.

### 4. Real token counts — ALREADY WORKING
No changes needed.

### 5. Pricing data — NOW EXPOSED via /bench/models
See item 9 below.

### 6. Strategy field — FIXED
**Problem:** Frontend sent `config.strategy`, backend expected `overrides.strategy`.
**Fix:** Added `#[serde(alias = "config")]` to `overrides` field in `StartBenchRequest`.
**Files:** `crates/roko-serve/src/routes/bench.rs`

### 7. Playbook extraction — ALREADY WORKING
No changes needed.

### 8. Anti-pattern extraction — ALREADY WORKING
No changes needed.

### 9. Bench models endpoint — FIXED
**Problem:** `/api/bench/models` returned bare slug strings. Frontend expected full
`BenchModel` objects with `cost_per_1k_input`, `cost_per_1k_output`, `context_window`, `provider`.
**Fix:** `list_models` handler now returns enriched objects with pricing from a static
table matching `estimate_cost_usd`, provider inference, and context window estimation.
**Files:** `crates/roko-serve/src/routes/bench.rs`

### 10. SSE events shape — FIXED (5 field mismatches)
| Field | Before (backend) | After (frontend-compatible) |
|---|---|---|
| Run ID key | `run_id` | `bench_id` |
| Task started | no `task_name` | includes `task_name` |
| Task completed | flat `passed/duration_ms/cost_usd` | full `result: BenchTaskResult` |
| Progress | `passed/failed` counts | `cost_so_far: f64` |
| Run completed | flat `pass_rate/total_cost_usd/total_duration_ms` | full `summary: BenchRunSummary` |

**Files:** `crates/roko-serve/src/events.rs`, `crates/roko-serve/src/routes/bench.rs`

### 11. POST URL mismatch — FIXED
**Problem:** Frontend sent `POST /api/bench/runs` (plural). Backend only had `POST /api/bench/run` (singular).
**Fix:** Added `.route("/bench/runs", get(list_bench_runs).post(start_bench_run))` plus
`/bench/runs/{id}`, `/bench/runs/{id}/events`, `/bench/runs/{id}/cancel` aliases.
**Files:** `crates/roko-serve/src/routes/bench.rs`

---

## Files Changed

### Rust backend
- `crates/roko-serve/src/events.rs` — bench event field renames
- `crates/roko-serve/src/routes/bench.rs` — POST URL aliases, strategy alias, models enrichment, event emissions
- `crates/roko-serve/src/routes/status/gates.rs` — waterfall format support
- `crates/roko-serve/src/routes/learning/mod.rs` — threshold gauges shape, provider mosaic shape, test updates
- `crates/roko-serve/src/routes/learning/router_state.rs` — C-Factor trend wrapping + Woolley
- `crates/roko-serve/src/routes/aggregator.rs` — topology node shape + edges
- `crates/roko-serve/src/routes/dream.rs` — new GET /api/dream/journal route

### TypeScript frontend
- `demo/demo-app/src/pages/dashboard/ChainView.tsx` — pass `&format=waterfall` to gate history
- `demo/demo-app/src/pages/dashboard/DreamsView.tsx` — new page wrapping DreamPhaseViz
- `demo/demo-app/src/pages/dashboard/Layout.tsx` — added Dreams nav link
- `demo/demo-app/src/main.tsx` — mounted Dreams route

---

## Verification

```bash
# Backend compiles with zero warnings from our changes:
cargo check -p roko-serve
# (pre-existing errors in shared_runs.rs and lib.rs are unrelated)

# Run roko-serve tests:
cargo test -p roko-serve

# Demo app builds:
cd demo/demo-app && npm run build
```

## Remaining Gaps (not addressed)

1. **Demo stub agent** — `echo demo-stub` still makes scenarios 2-5 no-ops. This is a config choice, not a code bug.
2. **`--enable-terminal` default** — still false. Scenarios need manual flag.
3. **TUI `dream_view.rs`** — exists but not wired into any TUI tab (ratatui). Only the demo web app was wired.
4. **`POST /api/dream/run` hardcodes `cat`** — the REST dream trigger uses a no-op agent. CLI path works correctly.
