# Frontend/API Parity

The React demo app (`demo/demo-app/`) is a real surface that talks to `roko-serve`
(control plane on `:6677`, ~200 distinct route paths). It is **not yet a route-contract
authority**: some pages use the `DataHub` cache layer, some use the transport `api`
singleton, some use per-domain `lib/*-api.ts` clients, and some open raw
`WebSocket`/`EventSource` with hand-written path strings. This document maps every
frontend call to its serve route (or flags the miss) with `file:line` evidence, verified
against the current tree (`demo/demo-app/src`, `crates/roko-serve/src/routes`).

## Route mounting model (verified)

`roko-serve` assembles almost every domain router with `.merge(...)` into one `api`
router, then mounts the whole thing at `.nest("/api", api)`
(`crates/roko-serve/src/routes/mod.rs:148-191,246`). **Consequence: nearly every route
string in the codebase that looks like `/plans`, `/bench/...`, `/workflow/events` is
actually served at `/api/plans`, `/api/bench/...`, `/api/workflow/events`.** The frontend
correctly prepends `/api` in almost all callers.

Routes mounted **outside** `/api` (top level, `mod.rs:233-250`):

| Path | Handler | Notes |
|---|---|---|
| `/health`, `/ready`, `/metrics` | liveness/readiness/Prometheus | No auth, no `/api` prefix. |
| `/ws`, `/roko-ws` | `ws::routes()` (`routes/ws.rs:24-25`) | `ServerEvent` WebSocket. |
| `/ws/terminal/{id}`, `/api/terminal/sessions` | terminal PTY (`terminal.rs:1120-1151`) | Config + bind gated. |
| `/relay`, `/relay/{*path}`, `/relay/agents/ws`, `/relay/events/ws` | `relay_proxy::routes()` (`routes/relay_proxy.rs:26-30`) | Proxy to native agent-relay. |
| webhooks public, shared-run public | `webhooks::public_routes()`, `shared_runs::public_routes()` | No auth. |

**Note:** `/workflow/events` is added to the `api` router (`mod.rs:191`), so it resolves at
`/api/workflow/events` — the frontend's `/api/workflow/events` (`useEventStream.ts:196`)
is correct despite the un-prefixed handler name.

## Frontend API client layers (verified)

| Layer | File | Role | Status |
|---|---|---|---|
| `DataHub` | `src/app/DataHub.ts` (1255 LOC) | Centralized cache/data layer. Calls `/api/config`, `/api/bench/*`, `/api/managed-agents`, `/api/workspaces*`, `/api/isfr/*`, `/api/chain/*`, `/api/feeds/catalog`. | Partial migration; ~18 routes covered. |
| `api` singleton | `src/transport/api.ts:112` (`RokoApi`) | Never-throws `ApiResult<T>` fetch wrapper. Base URL from `lib/serve-url.ts`. Caller supplies full `/api/...` path. | Preferred transport. |
| `useApi()` hook | `src/hooks/useApi.ts` | **`@deprecated`** in favor of `transport/api`. Still exported. | Legacy. |
| `useEventStream` | `src/hooks/useEventStream.ts` | SSE manager; opens `/api/events` (unnamed) + `/api/workflow/events` (named) (`:195-196`). | Real; reconnect w/ backoff. |
| `transport/sse.ts` | SSE client with `Last-Event-ID` replay | Separate SSE impl from `useEventStream` (duplication). | See drift below. |
| `transport/ws.ts` | Generic WS adapter (`:89`) | Not universal; pages still open raw `WebSocket`. |  |
| `lib/isfr-api.ts` | ISFR REST + SSE | `/api/isfr/status\|current\|history\|sources` + EventSource `/api/isfr/stream`. |  |
| `lib/relay-api.ts` | Relay REST | `/relay/agents\|workspaces\|feeds\|topics\|health`. | Response types drift (see below). |
| `lib/chain-api.ts` | Chain REST | `/api/chain/blocks\|transactions\|events\|watcher`. |  |
| `lib/feeds-api.ts` | `/api/feeds/catalog`. |  |  |
| `lib/workflow-api.ts` | Workflow SSE + WS | EventSource `/api/workflows/latest/stream` (`:190`), WS `/api/workflow/ws` (`:236`). | Both routes exist. |

## Broken or mismatched calls (verified)

| Frontend call | Caller (file:line) | Server reality | Verdict |
|---|---|---|---|
| `GET /api/share/{token}` | `pages/Share.tsx:103` | Server exposes **`/api/shared/{token}`** (`shared_runs.rs:127`). | **404 — path typo.** `share` vs `shared`. |
| `GET /api/bench/matrix` | `hooks/useMatrixBench.ts:151` | No `bench/matrix` route in `bench.rs`. Matrix data arrives only via SSE bench events (`MatrixRunStarted`…). | **404 — no REST route.** |
| `EventSource /api/isfr/stream` | `lib/isfr-api.ts:113` | No `isfr/stream` route; `isfr::routes()` has only `current/history/sources/status`. | **404 — no SSE route.** |
| `WebSocket /ws/agents` | `pages/isfr/IsfrTabDrawer.tsx:101` | No `/ws/agents`. Server has `/ws`, `/roko-ws`, `/relay/agents/ws`. | **WS close — wrong path.** Likely meant `/relay/agents/ws`. |
| `WebSocket url` (dynamic) | `pages/dashboard/RelayDashboard.tsx:39` | Resolves through `/relay/events/ws` proxy or direct relay; canonical source undecided. | Works but ambiguous. |
| `/relay/health\|feeds\|topics` helper shapes | `lib/relay-api.ts:70-96` | Proxy returns native relay JSON; TS helper response types are hand-written and drift from actual relay shapes. | Shape drift. |
| Mirage `/api/ws`, chain sim WS | `hooks/useChain.ts:128` (`MIRAGE_EVENTS_WS_URL`), `useBlockStream.ts:125` | Mirage-specific; **not** a `roko-serve` route. | Out of scope — belongs to mirage-rs. |
| SSE `lastEventId` query param | `transport/sse.ts:111` | Server reads **`Last-Event-ID` header** only (`routes/sse.rs:42-43`); ignores the query param. | Silent no-op on replay. |

### Confirmed-good calls (previously suspect)

| Call | Caller | Route | Status |
|---|---|---|---|
| `/api/workflow/ws` | `workflow-api.ts:236`, `IsfrDashboard.tsx:936` | `workflows.rs:41` → `/api/workflow/ws` | **Exists.** WS replay/filter still unproven. |
| `/api/workflow/events` | `useEventStream.ts:196` | `mod.rs:191` (in `api` router) | **Exists.** |
| `/api/agents/topology` | DataHub | `aggregator.rs:43` | **Exists.** |
| `/api/knowledge/entries\|edges` | DataHub / knowledge UI | `aggregator.rs:55-56` | **Exists.** |
| `/api/c-factor/trend`, `/api/metrics/c_factor` | metrics UI | `learning/mod.rs:26`, `status/mod.rs:25` | **Both exist** (two different C-Factor endpoints). |
| `/api/statehub/events\|snapshot` | dashboard | `status/mod.rs:44-45` | **Exists.** |
| `/api/gates/history\|summary` | gates UI | `status/mod.rs:34-35` | **Exists.** |

## Cross-cutting: `/learn/*` vs `/learning/*` dual namespace

Serve mounts **both** `/api/learn/<x>` and `/api/learning/<x>` to the same handlers for
most learning endpoints (`routes/learning/mod.rs:27-49`), a deliberate compatibility shim.
The frontend uses **both** prefixes inconsistently (`/api/learn/efficiency`,
`/api/learn/cascade-router`, `/api/learn/router`, `/api/learning/gate-thresholds`). Not a
bug today, but a drift trap: a few endpoints are aliased on only one side (e.g.
`/learning/cascade` and `/learn/cascade` both exist, but confirm before assuming symmetry).
Pick one canonical prefix and delete the other.

## Status matrix

| Surface area | Frontend caller | Serve owner | Parity |
|---|---|---|---|
| Health/status/events | `api`, `useEventStream` | `status/`, `sse` | Wired |
| Plans / workflows | `workflow-api`, DataHub | `plans.rs`, `workflows.rs` | Wired (WS semantics unproven) |
| Learning / C-Factor / gates | DataHub, metrics UI | `learning/`, `status/metrics.rs`, `status/gates.rs` | Wired (dual `learn`/`learning` prefix) |
| Bench (REST) | `Bench.tsx`, DataHub | `bench.rs`, `swe_bench.rs` | Wired |
| Bench matrix (REST) | `useMatrixBench.ts:151` | — | **Missing (404)** |
| ISFR REST | `lib/isfr-api.ts` | `isfr.rs` | Wired |
| ISFR SSE stream | `lib/isfr-api.ts:113` | — | **Missing (404)** |
| ISFR agent WS | `IsfrTabDrawer.tsx:101` | — | **Missing (`/ws/agents`)** |
| Chain / knowledge / neuro | `chain-api`, DataHub | `chain.rs`, `aggregator.rs`, `neuro.rs` | Wired |
| Workspaces / config / managed-agents | DataHub | `workspaces.rs`, `config.rs`, `aggregator.rs` | Wired |
| Feeds | `feeds-api` | `feeds.rs` | Wired |
| Relay | `relay-api` | `relay_proxy.rs` (proxy) | Wired but shape-drifted |
| Shared run reader | `Share.tsx:103` | `shared_runs.rs:127` | **Broken (`share` vs `shared`)** |
| Mirage chain sim | `useChain`, `useBlockStream` | mirage-rs (external) | Out of scope |

## Drift list (route-level)

1. `share` vs `shared` token path (`Share.tsx:103`).
2. `/api/bench/matrix` invented by frontend (`useMatrixBench.ts:151`).
3. `/api/isfr/stream` invented by frontend (`isfr-api.ts:113`).
4. `/ws/agents` invented by frontend (`IsfrTabDrawer.tsx:101`).
5. `lastEventId` query param ignored by server; only `Last-Event-ID` header replays.
6. `/learn/*` vs `/learning/*` split with partial-only aliasing.
7. Relay helper response types hand-written, drift from proxy passthrough.
8. Two SSE implementations (`useEventStream.ts` + `transport/sse.ts`) with duplicated
   event-name lists — see 76-DATA-CONTRACTS-SCHEMAS.md.
9. Frontend event field casing (`runId`, `gateFailure`, `prompt`) does not match server
   wire format (`run_id`, snake_case, `prompt_preview`) — see 76.

## DataHub migration checklist

- [ ] Fix the 4 hard 404s: `share→shared`, remove/implement `bench/matrix`,
      `isfr/stream`, `ws/agents`.
- [ ] Collapse `useApi` (deprecated) callers onto `transport/api`.
- [ ] Make DataHub consume a route manifest generated from `roko-serve` route assembly.
- [ ] Pick one canonical learning prefix (`/api/learn` or `/api/learning`); delete the alias.
- [ ] Keep Mirage routes (`useChain`, `useBlockStream`) in a clearly-labeled Mirage client.
- [ ] Keep relay routes in a relay client and mark serve `/relay/*` as the canonical proxy;
      generate relay response types from the relay crate, not by hand.
- [ ] Replace raw `new WebSocket("...")`/`new EventSource("...")` strings with named
      transport endpoints (compile-time route constants).
- [ ] Align SSE replay: either make server read `lastEventId` query or remove the frontend
      query behavior; keep `Last-Event-ID` header as the single mechanism.
- [ ] Add an E2E route smoke that mounts every page and fails on 404 / WS-close for
      required routes.

## Suggested route ownership

| Surface | Owns |
|---|---|
| `roko-serve` (`/api/*`) | health, status, events/SSE, plans, workflows, learning, gates, jobs, bench, swe-bench, config, workspaces, terminal, statehub, ISFR, chain, knowledge/neuro, feeds, agents, deployments, secrets. |
| `roko-serve` top-level | `/health`, `/ready`, `/metrics`, `/ws`, `/roko-ws`, `/ws/terminal/{id}`. |
| `agent-relay` | native `/relay/*` routes; serve exposes them via `relay_proxy` (canonical proxy). |
| `mirage-rs` | `/api/ws` chain simulation, pheromones, chain knowledge, Mirage prediction/task routes — **never** invented on `roko-serve`. |
| Frontend DataHub | Consumer only; must not invent endpoints. |

## Proof gate (ordered roadmap)

1. Extract registered serve routes from route assembly (walk `.route`/`.nest`, prefix `/api`).
2. Extract frontend path strings from `api.get/post/put/delete`, `fetch`, `new EventSource`,
   `new WebSocket`, and `lib/*-api.ts` clients.
3. Normalize `/api` nesting and `{param}` placeholders.
4. Partition explicitly-external Mirage and relay paths.
5. Fail CI on any frontend path with no owning serve route (would have caught all 4 live 404s).
6. Generate a typed client (or route-constant module) from the serve route manifest so path
   strings cannot drift.
