# E10 — Frontend / API Contract

> Executable backlog epic · verified against HEAD 5852c93c05 · sources: `105-FRONTEND-DEMO-APP`, `66-FRONTEND-API-PARITY`, `76-DATA-CONTRACTS-SCHEMAS`, `43-SURFACES-DEMO-UX`
> Native task schema: `crates/roko-cli/src/task_parser.rs::TaskDef` · exemplars: `plans/P11-runner-v2-default/tasks.toml`
> **Depends on: E03** (shared event/snapshot types — the snake_case wire contract in `E10-T05` must reference E03's canonical `DashboardEvent` shape, not a frontend-local copy).
> **Surface note:** this epic is the **web** demo (`demo/demo-app`, Vite+React+TS, embedded-served by roko-serve via the `rust-embed` fallback at `crates/roko-serve/src/routes/mod.rs:250`). It is **not** `P18-tui-agent-data`, which is the ratatui TUI — a different surface with its own transport. Do not merge the two.

## Why this epic

The web dashboard is the operator's demo-day face of roko, and its wire contract with `roko serve` is
**broken in four measurable ways**. Four frontend calls hit hard 404s (share, bench-matrix, isfr-stream,
ws-agents). Server events serialize snake_case (`prompt_preview`) while frontend types expect camelCase, so
some events **silently fail to deserialize**. The DataHub (Zustand) migration is half-finished: `main.tsx`
still mounts the deprecated `EventStreamProvider` **alongside** `bootstrapTransport()`, so **two independent
SSE managers** both connect to `/api/events` with divergent parsers. And reconnect replay is a coin-flip
because `transport/sse.ts` sends a `?n=` query param the server never reads (server honors only the
`Last-Event-ID` header). The result: a dashboard that looks live but drops events, double-connects, and 404s
on four panels.

## The one fix that matters

**Delete the deprecated `EventStreamProvider` and collapse to one SSE manager (E10-T06).**
`main.tsx:85` calls `bootstrapTransport()` which opens SSE `→ /api/events → DataHub` (`app/bootstrap.ts:43`),
and `main.tsx:93` *also* mounts `<EventStreamProvider>` which opens a *second* `EventSource` to the same
`/api/events` via `createEventStreamManager` (`contexts/EventStreamContext.tsx:27`). Two sockets, two parsers,
two sources of truth — every event is processed twice with different casing assumptions. Removing the legacy
provider (and migrating its 24 consumers to `useDataHub` selectors) is what makes the casing fix (E10-T05) and
the replay fix (E10-T07) meaningful instead of half-applied. Everything else in this epic is a route the DataHub
path will then own cleanly.

## Contract-break map (frontend call → server route → gap)

| # | Frontend call | Called at | Server owns | Owned at | Gap → task |
|---|---|---|---|---|---|
| 1 | `GET /api/share/{token}` | `pages/Share.tsx:103` | `GET /api/shared/{token}` | `routes/shared_runs.rs:874` | **404** — name mismatch (`share` vs `shared`) → **E10-T01** |
| 2 | `WS /ws/agents` | `pages/isfr/IsfrTabDrawer.tsx:101` | `/ws`, `/roko-ws` only | `routes/ws.rs:24-25` | **404** — no `/ws/agents` alias → **E10-T02** |
| 3 | `POST /api/bench/matrix` | `hooks/useMatrixBench.ts:151` | `MatrixRun` engine exists, **no route** | `bench.rs:1432` types + `save/load/list_matrix_run`; `routes/bench.rs` exposes none | **404** — engine built, route unwired → **E10-T03** |
| 4 | `GET /api/isfr/stream` (SSE) | `lib/isfr-api.ts:113` | `/isfr/status,current,history,sources` only | `routes/isfr.rs:23-26` | **404** — no streaming route → **E10-T04** |
| 5 | casing: `prompt_preview` (wire) vs camelCase (fe types) | `events.rs:235` `#[serde(rename="prompt_preview")]` | snake_case wire (`events.rs:12,86` `rename_all="snake_case"`) | — | **deser-fail** — fe types diverge → **E10-T05** |
| 6 | two SSE managers on `/api/events` | `main.tsx:85` (DataHub) + `main.tsx:93` (`EventStreamProvider`) | one SSE handler | `routes/sse.rs:23` | **double-connect** — half-done Zustand migration → **E10-T06** |
| 7 | `?n=` reconnect query | `transport/sse.ts` (`n=` on reconnect) | reads `Last-Event-ID` header only | `routes/sse.rs:42-46` | **replay-drop** — query param ignored → **E10-T07** |

## The 4 route fixes (summary)

1. **share** → align **frontend** to the server name: `pages/Share.tsx` `/api/share/${token}` ⇒ `/api/shared/${token}`.
2. **ws/agents** → add **server** alias: `routes/ws.rs` `.route("/ws/agents", get(ws_upgrade))` (same dashboard-event upgrade handler).
3. **bench/matrix** → add **server** route + handler: `POST /bench/matrix` in `routes/bench.rs`, wiring the already-built `MatrixRun` engine (`bench.rs:1432/1473`).
4. **isfr/stream** → add **server** SSE route: `GET /isfr/stream` in `routes/isfr.rs`, filtering `isfr_*` `DashboardEvent`s off the StateHub broadcast.

## Task breakdown (E10-Txx)

| Task | Tier | Summary | Depends |
|---|---|---|---|
| **E10-T01** | mechanical | Align `pages/Share.tsx` fetch `/api/share/{token}` → `/api/shared/{token}` (server owns `shared`). Frontend-only rename | — |
| **E10-T02** | mechanical | Add server WS alias `/ws/agents` → `ws_upgrade` in `routes/ws.rs` so `IsfrTabDrawer` connects | — |
| **E10-T03** | integrative | Add `POST /api/bench/matrix` route + `start_matrix_run` handler in `routes/bench.rs`, wiring the built `MatrixRun` engine (`bench.rs`) | E01 |
| **E10-T04** | integrative | Add `GET /api/isfr/stream` SSE route in `routes/isfr.rs` streaming `isfr_rate_computed`/`isfr_source_health_changed`/`isfr_keeper_state_changed` events | — |
| **E10-T05** | focused | Adopt snake_case as the canonical wire; add ONE deserialization adapter in `transport/` mapping server events (incl. `prompt_preview`) to fe types; delete the camelCase type divergence | **E03** |
| **E10-T06** | integrative | Remove deprecated `EventStreamProvider` from `main.tsx`; delete legacy `useEventStream` manager; migrate 24 consumers to `useDataHub` selectors → one SSE manager | E10-T05 |
| **E10-T07** | mechanical | Make `routes/sse.rs` honor `?n=`/`?lastEventId=` query as a fallback to the `Last-Event-ID` header for `replay_from`, matching what `transport/sse.ts` sends | E10-T05 |

## First 3 tasks (executable TOML)

```toml
[meta]
plan = "E10-FRONTEND-CONTRACT"
total = 7
done = 0
status = "ready"
max_parallel = 2

# ─────────────────────────────────────────────────────────────────────────────
# E10-T01: Fix the /api/share 404 — align frontend to server's /api/shared
# ─────────────────────────────────────────────────────────────────────────────
#
# pages/Share.tsx:103 fetches `/api/share/${token}` but the server route is
# `/api/shared/{token}` (routes/shared_runs.rs:874, handler get_shared_run).
# The client-side React route stays `share/:token` (main.tsx:119-120) — only the
# DATA fetch URL is wrong. One-word rename in the frontend; do NOT rename the
# server route (other callers may depend on `shared`).
#
[[task]]
id = "E10-T01"
title = "Align Share.tsx fetch to /api/shared/{token}"
status = "ready"
tier = "mechanical"
model_hint = "claude-haiku-4-5"
max_loc = 5
files = ["demo/demo-app/src/pages/Share.tsx"]
role = "implementer"
depends_on = []

[task.context]
read_files = [
    { path = "demo/demo-app/src/pages/Share.tsx", lines = "95-115", why = "The get<Receipt>(`/api/share/${token}`) call at :103 — the 404 site" },
    { path = "crates/roko-serve/src/routes/shared_runs.rs", lines = "860-880", why = "Server owns `/api/shared/{token}` → get_shared_run; this is the correct name" },
]
symbols = ["get_shared_run"]
anti_patterns = [
    "Do NOT rename the SERVER route to `share` — align the frontend to the existing `shared`.",
    "Do NOT touch the client-side React route `share/:token` in main.tsx — only the fetch URL is wrong.",
]

[[task.verify]]
phase = "structural"
command = "grep -q '/api/shared/' demo/demo-app/src/pages/Share.tsx && ! grep -q '/api/share/' demo/demo-app/src/pages/Share.tsx"
fail_msg = "Share.tsx must fetch /api/shared/{token}, not /api/share/{token}"

[[task.verify]]
phase = "structural"
command = "grep -q '/api/shared/{token}' crates/roko-serve/src/routes/shared_runs.rs"
fail_msg = "server route /api/shared/{token} must still exist as the contract target"

acceptance = "Loading /share/<token> in the demo fetches /api/shared/<token> and renders the transcript instead of a 404."


# ─────────────────────────────────────────────────────────────────────────────
# E10-T02: Fix the /ws/agents 404 — add a server WS alias
# ─────────────────────────────────────────────────────────────────────────────
#
# IsfrTabDrawer.tsx:101 opens `new WebSocket(`${WS_BASE}/ws/agents`)` but ws.rs
# only registers `/ws` and `/roko-ws` (routes/ws.rs:24-25), both → ws_upgrade.
# Add `/ws/agents` as a third alias to the SAME ws_upgrade handler so the drawer's
# agent stream connects. Do NOT fork a new handler — the upgrade already fans out
# dashboard/agent events.
#
[[task]]
id = "E10-T02"
title = "Add /ws/agents WS alias to ws_upgrade"
status = "ready"
tier = "mechanical"
model_hint = "claude-haiku-4-5"
max_loc = 6
files = ["crates/roko-serve/src/routes/ws.rs"]
role = "implementer"
depends_on = []

[task.context]
read_files = [
    { path = "crates/roko-serve/src/routes/ws.rs", lines = "20-40", why = "routes() registers /ws and /roko-ws → ws_upgrade; add /ws/agents alias here" },
    { path = "demo/demo-app/src/pages/isfr/IsfrTabDrawer.tsx", lines = "95-110", why = "Frontend opens WS at `${WS_BASE}/ws/agents` — the path the alias must match" },
]
symbols = ["ws_upgrade", "routes"]
anti_patterns = [
    "Do NOT write a new upgrade handler — reuse ws_upgrade (same dashboard-event fan-out).",
    "Do NOT nest under /api — ws routes live at /ws (frontend uses WS_BASE, not the /api prefix).",
]

[[task.verify]]
phase = "structural"
command = "grep -q '\"/ws/agents\"' crates/roko-serve/src/routes/ws.rs"
fail_msg = "ws.rs must register the /ws/agents route"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-serve 2>&1"
fail_msg = "roko-serve must compile after adding the /ws/agents alias"

acceptance = "With `roko serve` running, the ISFR tab drawer's WebSocket to /ws/agents upgrades (101) instead of 404-ing, and receives agent events."


# ─────────────────────────────────────────────────────────────────────────────
# E10-T03: Fix the /api/bench/matrix 404 — wire the built MatrixRun engine
# ─────────────────────────────────────────────────────────────────────────────
#
# useMatrixBench.ts:151 POSTs `/api/bench/matrix` to start a matrix (model×suite)
# run, but routes/bench.rs registers no matrix route at all. The engine already
# exists: bench.rs:1432 `struct MatrixRun`, :1473 `save_matrix_run`, :1480
# `load_matrix_run`, :1492 `list_matrix_runs`, and the SSE emits MatrixRunStarted/
# MatrixLaneCompleted/MatrixRunCompleted (events.rs:532-556). This is a WIRE task:
# add a `POST /bench/matrix` route + `start_matrix_run` handler that kicks the
# existing engine and returns the matrix_id, mirroring how `start_bench_run`
# (routes/bench.rs:36) drives a single run. Emit the matrix events on the same
# StateHub the /bench/events SSE already streams.
#
[[task]]
id = "E10-T03"
title = "Add POST /api/bench/matrix route wiring MatrixRun engine"
status = "ready"
tier = "integrative"
model_hint = "claude-sonnet-5"
max_loc = 120
files = ["crates/roko-serve/src/routes/bench.rs"]
role = "implementer"
depends_on = ["E01"]

[task.context]
read_files = [
    { path = "crates/roko-serve/src/routes/bench.rs", lines = "30-60", why = "routes() table + start_bench_run pattern to mirror for the matrix start handler" },
    { path = "crates/roko-serve/src/bench.rs", lines = "1400-1510", why = "MatrixRun type + save/load/list_matrix_run — the engine to drive" },
    { path = "crates/roko-serve/src/events.rs", lines = "530-560", why = "MatrixRunStarted/MatrixLaneCompleted/MatrixRunCompleted events to emit on StateHub" },
    { path = "demo/demo-app/src/hooks/useMatrixBench.ts", lines = "140-170", why = "The POST body/response shape the frontend expects from /api/bench/matrix" },
]
symbols = ["MatrixRun", "save_matrix_run", "list_matrix_runs", "start_bench_run", "MatrixRunStarted"]
anti_patterns = [
    "Do NOT reimplement matrix orchestration — call the existing MatrixRun engine in bench.rs.",
    "Do NOT invent a new event channel — publish MatrixRun* on the StateHub the /bench/events SSE already reads.",
    "Do NOT change the request shape unilaterally — match useMatrixBench.ts's POST body.",
]

[[task.verify]]
phase = "structural"
command = "grep -q '/bench/matrix' crates/roko-serve/src/routes/bench.rs"
fail_msg = "routes/bench.rs must register POST /bench/matrix"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-serve 2>&1"
fail_msg = "roko-serve must compile after adding the matrix route + handler"

[[task.verify]]
phase = "integration"
command = "grep -q '/api/bench/matrix' demo/demo-app/src/hooks/useMatrixBench.ts && grep -q '/bench/matrix' crates/roko-serve/src/routes/bench.rs"
fail_msg = "frontend call and server route must both name /bench/matrix (contract closed)"

acceptance = "POST /api/bench/matrix returns a matrix_id (200), a MatrixRun is persisted via save_matrix_run, and MatrixRunStarted appears on /api/bench/events."
```

## Remaining tasks (E10-T04 .. E10-T07)

Authored in the same schema when scheduled; key parameters:

- **E10-T04** (integrative, `routes/isfr.rs` + StateHub): add `GET /isfr/stream` SSE mirroring `routes/sse.rs`'s handler, filtered to `isfr_rate_computed`/`isfr_source_health_changed`/`isfr_keeper_state_changed`. Verify: `grep -q '/isfr/stream' crates/roko-serve/src/routes/isfr.rs`; `curl -sN localhost:6677/api/isfr/stream` yields `data:` frames; frontend `lib/isfr-api.ts:113` call resolves.
- **E10-T05** (focused, **depends E03**, `transport/types.ts` + one `transport/` adapter): declare snake_case as canonical wire, add a single `fromWire()` mapper (handling `prompt_preview` → fe field) referencing E03's shared `DashboardEvent`, delete camelCase duplicates. Verify: `grep -rc 'promptPreview' demo/demo-app/src` → 0; a unit test round-trips a server `RunStarted` JSON through the adapter without dropping `prompt`.
- **E10-T06** (integrative, `main.tsx` + `contexts/EventStreamContext.tsx` + `hooks/useEventStream.ts` + 24 consumers): remove `<EventStreamProvider>` and `createEventStreamManager`; migrate consumers to `useDataHub`. Verify: `! grep -q 'EventStreamProvider' demo/demo-app/src/main.tsx`; exactly one `new EventSource` construction remains in the tree (`transport/sse.ts`); dashboard receives each event once.
- **E10-T07** (mechanical, `routes/sse.rs:42-46`): read `n`/`lastEventId` query param as a fallback to the `Last-Event-ID` header before `replay_from`. Verify: `curl -sN 'localhost:6677/api/events?n=5'` replays from seq 5; `grep -q 'lastEventId\|"n"' crates/roko-serve/src/routes/sse.rs`.
```
