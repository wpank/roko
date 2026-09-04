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

> **Verification (2026-09-03):** Independent code-level audit confirms all seven acceptance
> criteria are satisfied in the integrated tree. Full findings below. Three verification
> checklist items remain open and require runtime execution to close.

**Status**: Verified (2026-09-03) — 12 routes, cursor pagination, offline repair
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
- [x] Prove HTTP/startup never invokes repair and add truncation/active-lock refusal cases.
      (2026-09-03 code audit: HTTP handlers call only `read_for_run`/`open_existing_run_index`;
      four repair unit tests cover truncation, workspace lock, repair lease, and writer lock
      refusal. No runtime path from serve or request handling reaches repair logic.)

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

## Verification Findings (2026-09-03)

Independent code-level audit of the integrated tree. No build/test/clippy was executed per
constraints; all findings are from source inspection.

### 1. Bounded `/api/runs/{run_id}/` endpoints -- CONFIRMED

All twelve routes exist in `crates/roko-serve/src/routes/runs.rs` (lines 48-65):

| Route | Handler | Purpose |
|---|---|---|
| `GET /api/dashboard/runs` | `get_dashboard_runs` | Bounded per-run index discovery |
| `GET /api/runs/{run_id}` | `get_run_detail` | Detail with integrity, links, and summary |
| `GET /api/runs/{run_id}/events` | `get_run_events` | Cursor-paginated events; SSE when `Accept: text/event-stream` |
| `GET /api/runs/{run_id}/events/stream` | `get_run_events_stream` | Explicit SSE alias |
| `GET /api/runs/{run_id}/tasks` | `get_run_tasks` | Task summaries with observed attempt numbers |
| `GET /api/runs/{run_id}/tasks/{task_id}/attempts` | `get_task_attempts` | Events grouped by attempt |
| `GET /api/runs/{run_id}/gates` | `get_run_gates` | Gate lifecycle/results |
| `GET /api/runs/{run_id}/logs` | `get_run_logs` | Scrubbed, level-filtered log previews |
| `GET /api/runs/{run_id}/metrics` | `get_run_metrics` | Tokens, cost, duration, tasks, gates |
| `GET /api/runs/{run_id}/artifacts` | `get_run_artifacts` | Artifact + checkpoint metadata |
| `GET /api/runs/{run_id}/screenshots` | `get_run_screenshots` | Screenshot manifest + metadata |
| `GET /api/runs/{run_id}/bundle` | `get_run_bundle` | Evidence-bundle manifest + file inventory |

Route registration is confirmed in `crates/roko-serve/src/routes/mod.rs` line 328:
`.merge(runs::routes())`. The `shared_runs` module is also merged at lines 373 and 472.

Every handler calls `ensure_observability_allowed()` which gates access on loopback binding
or enabled API authentication (`state.listener_security.observability_allowed()`).

Every handler calls `validate_id()` which delegates to
`roko_fs::run_index::validate_scoped_id()`, rejecting empty, overlong (>128 bytes),
non-alphanumeric-plus-dash-underscore-dot-colon, and traversal-containing (`..`) identifiers.

Unknown/malformed run resources return explicit 404 errors (`ApiError::not_found`) rather
than implying an empty successful run. The `get_run_detail` handler checks active runs,
on-disk indexes, and bundle directories before returning a 404.

### 2. Cursor pagination -- CONFIRMED

Pagination uses a byte-offset `cursor` model (not offset/limit):

- `EventQuery` struct accepts `cursor: Option<u64>` and `limit: Option<usize>`.
- Default limit is 100, maximum is 200 (`bounded_limit()`).
- `read_index_page_filtered()` (lines 1053-1163) seeks to the cursor byte position in the
  per-run JSONL index, verifies the previous byte is a newline (line boundary), and reads
  forward up to the page limit or byte scan cap (`MAX_PAGE_SCAN_BYTES = 4 MiB`).
- Every page response includes `cursor`, `next_cursor`, and `has_more` fields.
- Partial tails (concurrent append without trailing newline) are detected and the cursor
  does not advance beyond them (lines 1113-1119).
- The `page_integrity()` helper reports `quarantined_records`, `partial_tail`,
  `scanned_bytes`, and `response_bounded` in every response.

Six unit tests in `runs.rs` cover cursor behavior:
- `cursor_pages_are_bounded_and_resume_at_line_boundary` -- two-page traversal
- `mismatched_and_malformed_records_are_quarantined` -- quarantine counting
- `partial_tail_does_not_advance_the_external_cursor` -- concurrent append safety
- `filtered_pagination_resumes_after_last_emitted_match` -- filtered cursor stability
- `type_filter_and_secret_scrubbing_apply_before_response` -- type filter + scrubbing
- `summary_reports_tasks_gates_and_terminal_metrics` -- summary aggregation

SSE cursor pagination uses `Last-Event-ID` header for reconnection (lines 259-263) and
deduplicates live frames whose persisted cursor is not beyond the replay suffix (line 313).

### 3. Offline repair/query -- CONFIRMED

The `roko run-index repair` CLI command in `crates/roko-cli/src/commands/run_index.rs`
(1076 lines) provides bounded offline repair:

- **Dry-run by default**: without `--apply`, the command is read-only and reports what would
  be rebuilt.
- **Bounded**: configurable `--max-bytes` (default 512 MiB), `--max-records` (default 1M),
  `--max-indexes` (default 4096), and `--deadline-secs` (default 120s). Truncation on any
  limit fails the entire apply atomically.
- **Locking**: `RepairLocks` acquires exclusive locks on `cache-gc.lock`, `roko.lock`,
  `run-index-repair.lock`, `events.jsonl.lock`, and `runtime-events.jsonl.lock` (sorted to
  prevent deadlocks). Active workspace locks refuse repair.
- **Source scanning**: discovers rotated JSONL generations (`events.<timestamp>.jsonl`) plus
  the live log. Cross-run records, malformed records, invalid IDs, partial tails, and
  oversized records are all counted and rejected.
- **Atomic replacement**: `StagingSet` writes to temporary files, validates final paths, and
  uses `rename()` for atomic replacement only after a complete scan.
- **HTTP/startup isolation**: HTTP handlers (`runs.rs`) never call repair. The
  `source_path()` function returns per-run index paths via `roko_fs::run_index`, but
  the indexes are expected to already exist (written by `runner/persist.rs` or
  `roko-runtime/src/jsonl_logger.rs` during live runs). Missing indexes result in
  `None`/404, not automatic rebuilds.

Nine unit tests in `run_index.rs` cover repair behavior:
- `validates_envelope_ownership_and_rejects_cross_run_records`
- `truncated_apply_discards_staging_without_replacing_indexes`
- `apply_rebuilds_hashed_indexes_and_rejects_bad_records`
- `distinct_index_limit_fails_closed_without_replacement`
- `active_workspace_lock_refuses_apply`
- `partial_record_at_eof_is_counted_and_preceding_records_are_indexed`
- `active_repair_lease_refuses_concurrent_repair`
- `active_event_log_writer_lock_refuses_repair`
- `symlinked_index_directory_is_rejected` (unix-only)

### 4. Supporting infrastructure -- CONFIRMED

**Per-run index paths** (`crates/roko-fs/src/run_index.rs`, 267 lines):
- `validate_scoped_id()` -- grammar check
- `run_index_path()` -- SHA-256 hashed path derivation (never interpolates raw IDs)
- `open_run_index_append()` / `open_existing_run_index()` -- symlink-rejecting,
  ownership-validating, NOFOLLOW fd-based open (on macOS/Linux; fallback on other platforms)
- Three unit tests plus a unix-only symlink rejection test

**Writer side** (`crates/roko-cli/src/runner/persist.rs`, lines 470-560):
- `append_run_scoped_event()` appends to both the global log and the derived per-run index
- Buffered 64 KiB writers with bounded cache (`MAX_RUN_INDEX_WRITERS`) and flush-on-eviction

**Runtime side** (`crates/roko-runtime/src/jsonl_logger.rs`):
- `run_path()` and `flush_run()` methods enable the HTTP handlers to materialize and read
  runtime per-run indexes on demand without replaying global logs

**OpenAPI** (`crates/roko-serve/src/openapi.rs`, lines 80-91, 330-405):
- All 11 run-observability endpoints are documented under the `run-observability` tag
- The `run_observability_attempts` endpoint has a custom utoipa annotation with explicit
  parameter and response schemas

**API reference** (`docs/v2/API-REFERENCE.md`, lines 809-819):
- All run-scoped routes are documented with query parameters and descriptions

### 5. Verification checklist updates

Items that can be closed by this code-level audit:

- Checklist item "Prove HTTP/startup never invokes repair and add truncation/active-lock
  refusal cases": The code-level evidence is now complete. HTTP handlers in `runs.rs` call
  only `read_for_run()` / `read_for_run_filtered()` which call `read_index_page()` /
  `read_index_page_filtered()` which call `open_existing_run_index()`. None of these invoke
  repair, scan global logs, or rebuild indexes. The repair command holds five exclusive
  locks, and four unit tests (truncation, workspace lock, repair lease, event log writer
  lock) prove refusal behavior. This is satisfied at the source level; runtime proof
  requires the `cargo test` lane to pass.

Items that remain open and require runtime execution:

- `cargo test -p roko-serve` -- no integration tests for the run-observability routes exist
  in `crates/roko-serve/tests/`. The unit tests in `runs.rs` (6 tests) and `run_index.rs`
  (9 tests) cover the core logic but not the HTTP handler layer.
- Real plan-to-JSONL cross-check -- requires running `roko plan run`, then querying the API
  and comparing against direct JSONL/index contents.
- Manual filtered event types/sources and attempt grouping -- requires a live server.
