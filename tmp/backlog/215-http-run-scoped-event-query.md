# 215 — HTTP Run-Scoped Event and Gate Query Endpoints

> **Status: DONE FOR NEW RUNS / HISTORICAL REPAIR OPEN** (2026-08-31, `5f689d66e`). The
> implementation uses the safer run-resource surface under `/api/runs/{run_id}` rather than the
> draft global `/api/events?run_id=` shape. It provides bounded detail, cursor events, SSE,
> tasks/attempts, gates, scrubbed logs, metrics, artifact/screenshot inventories, and bundle
> metadata. Requests/startup deliberately do not rebuild pre-index history; a bounded offline
> repair command remains open.

**Priority**: P2 — events are only accessible by reading JSONL files directly; no HTTP query capability for run-scoped or filtered event access
**Size**: M (2-3 days)
**Crates**: `roko-serve`, `roko-cli`
**Depends on**: #212 (run_id in snapshots and events)
**Sources**: tmp/backlog/_mori-diffs-gaps.md Group G-2; 23-HANDOFF-OPEN-ITEMS.md section 06; 29-CURRENT-RUNTIME-GAP-LEDGER.md P1-04; 21-FEATURE-PARITY-MATRIX.md OB-01

## Background

Runtime events are written to `.roko/events.jsonl` but there is no HTTP endpoint to query them by run_id, task_id, event category, or gate rung. The "TUI/API/CLI one truth" parity goal requires that external tools and dashboards can query the same runtime data the TUI shows without reading JSONL files directly. Currently a proof script that wants to verify runtime behavior must parse raw JSONL files, which is fragile and couples the proof to the file format.

Once #212 lands (run_id in all persisted events), run-scoped queries become possible. This item adds the HTTP endpoints that consume that data.

## Audited baseline (historical)

- `roko-serve` exposes ~317 routes on `:6677` but none for querying runtime events or gate results by run
- Events are persisted to `.roko/events.jsonl` with ~40 `RunnerEvent` variants
- Gate results are embedded in `RunnerEvent::GateCompleted` variants
- The TUI reads events from JSONL directly via `crates/roko-cli/src/tui/state.rs`
- `roko-serve` has existing route patterns in `crates/roko-serve/src/routes/` that can be followed
- SSE endpoints exist for live streaming but not for historical event queries
- #212 will add `run_id` to every persisted event, enabling run-scoped filtering

## Implementation Plan

1. **Add an event index layer** in `roko-serve` or `roko-cli`:
   - On startup (or lazy first access), scan `.roko/events.jsonl` and build an in-memory index by `run_id`, `task_id`, and event category
   - Use the `run_id` envelope from #212 for run-scoped indexing
   - Optionally support incremental index updates for live runs (tail the JSONL)

2. **Add REST endpoints** in `crates/roko-serve/src/routes/events.rs`:
   - `GET /api/events?run_id=<id>` — all events for a run, ordered by timestamp
   - `GET /api/events?run_id=<id>&task_id=<id>` — events for a specific task within a run
   - `GET /api/events?run_id=<id>&category=<cat>` — events filtered by category (dispatch, gate, lifecycle, learning)
   - `GET /api/gates?run_id=<id>` — all gate results for a run
   - `GET /api/gates?run_id=<id>&rung=<n>` — gate results filtered by rung number
   - All endpoints return JSON arrays with pagination (`?offset=0&limit=100`)

3. **Add event categories** — define an `EventCategory` enum that classifies each `RunnerEvent` variant:
   - `Dispatch` (task started, agent spawned, output received)
   - `Gate` (gate started, gate completed, gate failed)
   - `Lifecycle` (run started, run completed, resume marker)
   - `Learning` (efficiency event, episode recorded, router updated)
   - Map each `RunnerEvent` variant to its category

4. **Wire into serve startup** — register the event routes in the serve router alongside existing routes

5. **Add proof test** — an integration test that starts `roko serve`, runs a tiny plan, and queries events through HTTP to verify they match the JSONL file contents

## Acceptance Criteria

- [x] `GET /api/runs/{run_id}/events` returns bounded ordered events for the specified run.
- [x] `GET /api/runs/{run_id}/tasks/{task_id}/attempts` groups task-specific events by attempt.
- [x] Event `types`/`source` filters and dedicated gate/log routes provide scoped categories.
- [x] `GET /api/runs/{run_id}/gates` returns bounded gate lifecycle/results.
- [x] Pagination uses an opaque byte `cursor` plus a capped `limit`; offset scans are superseded.
- [x] Unknown/malformed run resources fail safely instead of implying an empty successful run.
- [ ] Pre-index events require an explicit bounded offline repair; they are never assigned to a
      shared `unknown` run by a live request.

## Verification Checklist

- [ ] `cargo build -p roko-serve` compiles on the final integrated tree — final batch pending.
- [ ] `cargo test -p roko-serve` passes, including new endpoint tests — final batch pending.
- [ ] Manual: start `roko serve`, run a plan, then query `/api/runs/{run_id}/events`.
- [ ] Manual: query `/api/runs/{run_id}/gates` and the run-filtered SSE route.
- [ ] Manual: query filtered event types/sources and attempt grouping.
- [ ] Response matches JSONL file contents when cross-checked

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-serve/src/routes/runs.rs` | Bounded run detail/events/SSE/tasks/gates/logs/metrics/artifact/screenshot/bundle routes |
| `crates/roko-serve/src/routes/shared_runs.rs` | Dashboard/shared-run discovery without global-log replay |
| `crates/roko-serve/src/openapi.rs` / `docs/v2/API-REFERENCE.md` | Document the safe run-scoped surface |
| `crates/roko-fs/src/run_index.rs` | Validate/hash IDs and derive per-run index paths |
| `crates/roko-cli/src/runner/persist.rs` | Project runner records into buffered per-run indexes |
| `crates/roko-runtime/src/jsonl_logger.rs` | Project canonical runtime records with cursor-safe buffering |
