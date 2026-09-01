# 215 — HTTP Run-Scoped Event and Gate Query Endpoints

> **Status: SOURCE-DONE; LOOPBACK API/CURSOR/SSE CHECKPOINT VERIFIED** (2026-08-31,
> `5f689d66e` + `85c052fc9`). The
> implementation uses the safer run-resource surface under `/api/runs/{run_id}` rather than the
> draft global `/api/events?run_id=` shape. It provides bounded detail, cursor events, SSE,
> tasks/attempts, gates, scrubbed logs, metrics, artifact/screenshot inventories, and bundle
> metadata. Requests/startup deliberately do not rebuild pre-index history. The separate
> dry-run-first `roko run-index repair` command scans recognized live/rotated sources under one
> aggregate budget and replaces indexes atomically only after a complete scan. The no-default
> serve check, disposable repair fixture, loopback run-resource endpoints, cursor traversal, and
> bounded SSE replay have passed. A real plan-to-JSONL cross-check remains open.

> **Status update (2026-09-01):** Run-index repair release fixtures (truncation and lock refusal)
> have been added, closing two items from the dev-audit verification checklist. The real
> plan-to-JSONL cross-check remains the terminal proof for this item.

**Status**: Verification only; close the real plan-to-JSONL cross-check before archive and do not redispatch the historical implementation plan
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
- [x] Pre-index events use explicit bounded offline repair and are never assigned to a shared
      `unknown` run by a live request; malformed/cross-run data and truncated scans fail closed.

## Verification Checklist

- [x] `cargo check -p roko-serve --no-default-features --locked -j1` passes on the integrated tree.
- [ ] `cargo test -p roko-serve` passes, including new endpoint tests, in the release/full-CI lane.
- [x] Manual: start `roko serve` against a bounded fixture and query status, run detail, events,
      tasks, gates, and metrics; health/readiness and every selected run endpoint returned `200`.
- [x] Manual: traverse event cursor positions `0` to `40` to `123` and replay bounded run-filtered
      SSE.
- [ ] Run a real plan and cross-check its API records against its direct JSONL/index records.
- [ ] Manual: query filtered event types/sources and attempt grouping.
- [x] Manual: exercise `run-index repair` dry-run/apply with valid, malformed, invalid-ID, and
      cross-run records.
- [ ] Prove HTTP/startup never invokes repair and add truncation/active-lock refusal cases.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-serve/src/routes/runs.rs` | Bounded run detail/events/SSE/tasks/gates/logs/metrics/artifact/screenshot/bundle routes |
| `crates/roko-serve/src/routes/shared_runs.rs` | Dashboard/shared-run discovery without global-log replay |
| `crates/roko-serve/src/openapi.rs` / `docs/v2/API-REFERENCE.md` | Document the safe run-scoped surface |
| `crates/roko-fs/src/run_index.rs` | Validate/hash IDs and derive per-run index paths |
| `crates/roko-cli/src/runner/persist.rs` | Project runner records into buffered per-run indexes |
| `crates/roko-runtime/src/jsonl_logger.rs` | Project canonical runtime records with cursor-safe buffering |
| `crates/roko-cli/src/commands/run_index.rs` | Bounded dry-run-first historical index repair |
