# Data Contracts And Schemas

This ledger tracks the schemas that connect **runtime → server → frontend →
persistence → docs**. Every path and claim below is verified against the current tree
(`crates/`, `demo/demo-app/src`). The headline problems: (1) the same conceptual type is
defined in 3-7 crates with **no conversions** between them, and (2) the frontend TypeScript
event union is hand-written in a casing style that does not match the Rust wire format.

## Canonical candidates (verified file:line)

| Contract | Owner | Role | Drift risk |
|---|---|---|---|
| `RuntimeEventEnvelope` | `roko-core/src/runtime_event.rs:12` | Workflow/runtime envelope, `schema_version: 1` (`:16,32`), `#[serde(tag="kind", content="data")]` (`:63`). | Server/frontend use `ServerEvent`, not this directly; no bridge test. |
| `ServerEvent` | `roko-serve/src/events.rs:87` | SSE/WS union, `#[serde(tag="type", rename_all="snake_case")]` (`:86`). Core events snake_case; **bench/matrix/swe variants force PascalCase** via `#[serde(rename)]` (`:463-613`). | Hand-maintained TS mirror drifts; mixed tag casing. |
| `DashboardEvent` / `DashboardSnapshot` | `roko-core/src/dashboard_snapshot.rs` | StateHub materialized dashboard state. | Some event variants update snapshot, others only logged. |
| `StateHub` | `roko-runtime/src/state_hub.rs` | Live dashboard event application + persistence. | **Stale duplicate** at `roko-core/src/state_hub.rs` — do NOT treat as canonical. |
| `StateSnapshot` | `roko-runtime/src/state_snapshot.rs` | Versioned/checksummed runner snapshot. | Inner executor/orchestrator/run/gate payloads are opaque JSON strings owned elsewhere. |
| `RunLedger` | `roko-runtime/src/run_ledger.rs` | Runtime summary projection; not fully wired into WorkflowEngine. | Looks canonical before it is the live ledger. |
| `RunnerEvent` | `roko-cli/src/runner/types.rs` | Durable runner event vocabulary (v2). | Shares `.roko/events.jsonl` with other schemas. |
| Graph types | `roko-graph/src/types.rs` | Durable graph TOML/schema types. | Runtime `NodeResult`/`GraphOutput` are not durable serde contracts. |
| Route-local DTOs | `roko-serve/src/routes/*` | Request/response structs live beside handlers, many `private`. | OpenAPI/frontend types partial and manual. |
| Frontend event union | `demo/demo-app/src/transport/types.ts` | Consumer-side TS `ServerEvent`. | Hand-written, **camelCase — mismatches server snake_case** (see below). |

## Duplicated / drifted Rust types (VERIFIED — the core finding)

### `GateVerdict` — defined 4× with NO conversions (worse: 7 gate-verdict types)

| Definition | File:line |
|---|---|
| `struct GateVerdict` | `roko-core/src/foundation.rs:368` |
| `struct GateVerdict` | `roko-core/src/dashboard_snapshot.rs:290` |
| `struct GateVerdict` | `roko-learn/src/episode_logger.rs:90` |
| `struct GateVerdict` | `roko-chain/src/identity_economy_identity.rs:1600` |
| `struct GateVerdictSummary` | `roko-cli/src/runner/types.rs:141` |
| `struct GateVerdictSummary` | `roko-runtime/src/event_bus.rs:75` |
| `struct GateVerdictRecord` | `roko-core/src/forensic.rs:124` |

There are **no `From`/`Into` conversions** between any of these (grep for
`impl From<GateVerdict>` / `for GateVerdict` returns only an unrelated `roko-dreams/replay.rs`
loop). Each subsystem re-derives serde independently, so a gate result crossing the
foundation → learn → dashboard → chain boundary is re-parsed field-by-field, not converted.
This is the single highest-value consolidation target.

### `RetentionPolicy` — defined 3×

| Definition | File:line |
|---|---|
| `struct RetentionPolicy` | `roko-fs/src/gc.rs:32` |
| `struct RetentionPolicy` | `roko-serve/src/retention.rs:20` |
| `struct RetentionPolicy` | `roko-learn/src/episode_logger.rs:90` (line 1229) |

No shared crate; each owns its own retention semantics for the same conceptual policy.

### `StateHub` — canonical + stale duplicate

`roko-runtime/src/state_hub.rs` (live) vs `roko-core/src/state_hub.rs` (stale). Quarantine
the core copy.

## Frontend ↔ server event drift (VERIFIED)

Server serializes core `ServerEvent` variants in **snake_case** with renamed fields.
Frontend `transport/types.ts` declares the union in **camelCase** with different field
names. These cannot deserialize each other's payloads without a manual adapter; today the
frontend appears to read fields that the server never emits.

| Concept | Server wire (`events.rs`) | Frontend TS (`types.ts`) | Verdict |
|---|---|---|---|
| Run start tag | `run_started` | `run_started` (`:53`) | tag ok |
| Run start prompt | `prompt_preview` (renamed, `:235-236`) | `prompt` (`:53`) | **field mismatch** |
| Run id | `run_id` (`:234`) | `runId` (`:53`) | **casing mismatch** |
| Task failed | `task_failed` + snake fields | `gateFailure: boolean` (`:30`) | **casing mismatch** |
| Inference | `run_id` | `runId` (`:45-50`) | **casing mismatch** |
| Task ids | `task_id` | `taskId` (`:4-9,25-42`) | **casing mismatch** |
| Bench/matrix/swe tags | PascalCase (`BenchRunStarted`, `MatrixRunStarted`, `SweRunStarted`, `:463-613`) | mixed | tag-style split |

The whole `transport/types.ts` union is camelCase (`taskId`, `planId`, `agentId`, `opId`,
`runId`) while the server is snake_case — so this is **systemic**, not a one-off. Either the
server needs a camelCase serialization pass for the frontend, or the frontend types must be
generated from the Rust enums.

## Event tag-style split (VERIFIED)

- `RuntimeEventEnvelope`: `tag="kind"`, `content="data"`, snake_case (`runtime_event.rs:63`).
- `ServerEvent`: `tag="type"`, snake_case for core, PascalCase renames for bench/swe/matrix.
- `RunnerEvent` (runner v2): dotted `type` vocabulary.
- Workflow SSE emits `{ kind, run_id, data }`.
- Frontend `ServerEvent` union: `type`, camelCase.

Four envelopes, three tag keys (`kind`/`type`/dotted), two casing conventions.

## `.roko/events.jsonl` — contested file (VERIFIED)

Written/read by many crates: `roko-cli/src/runner/persist.rs`, `runner/resume.rs`,
`runner/event_loop.rs`, `serve_runtime.rs`, `run.rs`, plus TUI tailers
(`tui/jsonl_tailer.rs`, `jsonl_cursor.rs`, `tui/app.rs`) and e2e tests. StateHub appends raw
`DashboardEvent`; Runner v2 persists `RunnerEvent`. **Mixed schemas on one JSONL path with
no discriminating envelope** — a consumer must know which writer produced each line.

## SSE resume drift (VERIFIED)

- Server reads the `Last-Event-ID` **header** for replay (`routes/sse.rs:42-43`,
  `.replay_from(last_event_id)` `:52`).
- Frontend `transport/sse.ts:111` additionally appends a `lastEventId` **query param** that
  the server never reads. Silent no-op.
- Two frontend SSE implementations coexist (`transport/sse.ts` and
  `hooks/useEventStream.ts`) with duplicated event-name lists.

## Migration target

| Area | Target |
|---|---|
| Gate verdict | One shared `GateVerdict` in `roko-core`; all others become `From`/`Into` adapters or aliases. |
| Retention | One `RetentionPolicy` in a shared crate (`roko-core` or `roko-fs`); serve/learn re-export. |
| Events | One canonical taxonomy w/ explicit bridges: RuntimeEvent → DashboardEvent → ServerEvent → TypeScript. |
| Casing | Server emits a single casing (snake or camel) for the frontend; TS generated from Rust. |
| Event files | One schema per JSONL, or a discriminated envelope when mixed is intentional. |
| Snapshots | Version every durable snapshot; reject incompatible versions with migration guidance. |
| API DTOs | Generate OpenAPI / route manifest from live route assembly + exported public DTOs. |
| Frontend | Generate TS types from server contracts, or validate hand-written types against fixtures. |

## Drift list (schema-level)

1. `GateVerdict` × 4 (+3 sibling types), zero conversions — `foundation`/`dashboard_snapshot`/`learn`/`chain`.
2. `RetentionPolicy` × 3 — `fs/gc`/`serve/retention`/`learn/episode_logger`.
3. `StateHub` canonical (`roko-runtime`) vs stale (`roko-core`).
4. Frontend camelCase union vs server snake_case wire (systemic).
5. `prompt` vs `prompt_preview` field rename not mirrored in TS.
6. Four event envelopes, three tag keys, two casings.
7. `.roko/events.jsonl` mixed-schema, no discriminator.
8. `StateSnapshot` inner payloads = opaque JSON strings with separate owners.
9. `lastEventId` query (frontend) vs `Last-Event-ID` header (server).
10. Two frontend SSE clients with duplicated event-name lists.
11. Route DTOs private to handlers → OpenAPI/frontend typing manual.

## Checklist (ordered)

- [ ] Consolidate `GateVerdict` into one `roko-core` type; convert the 6 others to `From` adapters. (P0)
- [ ] Consolidate `RetentionPolicy` into one crate; re-export. (P1)
- [ ] Quarantine stale `roko-core/src/state_hub.rs`; repoint refs to `roko-runtime`. (P1)
- [ ] Fix `runId`/`taskId`/`prompt` casing: generate `transport/types.ts` from Rust enums, or add a camelCase serde pass server-side. (P0)
- [ ] Add RuntimeEvent → ServerEvent/DashboardEvent bridge tests.
- [ ] Add TS fixture tests for every streamed event tag (incl. PascalCase bench/swe/matrix).
- [ ] Split `.roko/events.jsonl` writers/readers or wrap mixed lines in a discriminated envelope.
- [ ] Align SSE resume: support `lastEventId` query or remove frontend query; keep `Last-Event-ID` header canonical.
- [ ] Collapse the two frontend SSE implementations into one.
- [ ] Export/generate route DTO schemas for OpenAPI + typed client (replace doc-only `Value` responses).
- [ ] Version JSONL schemas: episodes, engrams, events, runtime-events, run-ledger, learning state, knowledge.
- [ ] Replace opaque `StateSnapshot` inner strings with versioned typed sub-schemas + adapter tests.
- [ ] Add a schema registry: path/channel → owner type → serde tag style → schema version → compat policy.
- [ ] Forbid adding a frontend interface without a server fixture or generated schema.
