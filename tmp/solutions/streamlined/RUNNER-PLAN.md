# Runner Plan: `demo-truth`

Date: 2026-04-28

## Goal

Create one focused runner at `tmp/runners/demo-truth/` that makes the demo app a truthful
proof surface.

Truthful means:

- live workflows visibly pass, fail, or skip;
- no mutation silently falls back to fake data;
- no `/api/*` miss is accepted as SPA HTML;
- bench/share/explorer/dashboard render coherent live data when `roko serve` is running;
- demo scenarios cannot start until their terminal/session prerequisites are ready;
- failed CLI/backend work is shown as failed, not "running" or "connected."

## Non-Goals

Do not include these in this runner:

- `ChatAgentSession`
- `dispatch_direct` rewrite
- full model/provider selection unification
- grounded PRD context packs
- security/auth default policy changes
- `orchestrate.rs` decomposition
- broad telemetry type migrations such as changing all `cost_usd` fields to `Option`
- gate-core schema changes such as adding a `skipped` field to every gate verdict

Those are real issues, but they are not the shortest path to a truthful demo UI.

## Recommendation

Use one runner, not three. Keep the primary runner to API/frontend/demo-truth plus a few small CLI
display/path fixes. Move cross-crate telemetry/gate/model architecture into later runners.

Target:

- 29 batches total
- 3 contract batches
- 26 implementation/proof batches
- parallelism: `--parallel 4`
- expected wall time: roughly 2-3 hours if batches stay scoped

## Dependency Graph

```text
Group 0: contracts
  |
  v
Group A: stop fake success
  |
  +--> Group B: bench/share/API alignment
  +--> Group C: dashboard/explorer coherence
  +--> Group D: terminal/demo scenario truth
  +--> Group E: small CLI truth
            |
            v
Group F: proof harness
```

Safe parallelism:

- Group 0 runs first.
- Group A runs second because it changes error/fallback behavior that other frontend batches rely on.
- Groups B, C, D, and E can run after A, but respect per-file conflicts inside each group.
- Group F runs last.

## Runner Invariants

Every batch should follow these rules:

- Do not add mock data to make a workflow look successful.
- Do not swallow failed mutations.
- Do not mark a run active unless the backend returned a real run id.
- Do not rely on terminal text scraping for final product state when an API/event contract exists.
- Do not change unrelated design/layout.
- Do not broaden this into chat architecture or runtime convergence.
- If a workflow cannot be made real in the batch, make it explicitly failed/skipped with useful
  evidence.

## Group 0: Contract Guardrails

These are runner context files, not app source files. They give later batches a single contract to
follow without creating early code conflicts.

### Z01 - API Contract Doc

Create `tmp/runners/demo-truth/context/API-CONTRACT.md`.

Canonical routes:

```text
POST   /api/bench/run             -> { id: string }
GET    /api/bench/run/:id         -> BenchRun
GET    /api/bench/run/:id/status  -> BenchRunStatus
DELETE /api/bench/run/:id         -> 204
GET    /api/bench/runs            -> { runs: BenchRunIndexEntry[], total, limit, offset }
GET    /api/bench/suites          -> { suites: BenchSuiteSummary[] }
GET    /api/bench/suites/:id      -> BenchSuite
GET    /api/bench/models          -> { models: string[] }
GET    /api/bench/events          -> SSE stream
GET    /api/shared/:token         -> RunTranscript | 404
POST   /api/runs/:id/share        -> { token, url, transcript }
GET    /api/health                -> full health payload
GET    /health                    -> { status: "ok" }
GET    /api/statehub/events       -> { events: EventEnvelope[], cursor, limit }
GET    /api/managed-agents        -> Agent[]
GET    /api/learn/efficiency      -> EfficiencyResponse
GET    /api/learn/cascade-router  -> CascadeState
GET    /api/metrics/c_factor      -> CFactorResponse
GET    /api/gates/summary         -> GatesSummary
GET    /api/status                -> StatusResponse
```

Decision: frontend adapts to the current backend bench singular detail route. Do not add plural
detail aliases unless a later API redesign explicitly chooses that.

Scope:

- `tmp/runners/demo-truth/context/API-CONTRACT.md`

### Z02 - Fallback Policy Doc

Create `tmp/runners/demo-truth/context/FALLBACK-POLICY.md`.

Policy:

- Data modes are `checking`, `live`, and `demo`.
- `live`: server reachable at `/health`; API failures are real failures.
- `demo`: server unreachable; GETs may use sample data with a visible demo-mode badge.
- mutations (`POST`, `PUT`, `PATCH`, `DELETE`) never fall back.
- `text/html` from an `/api/*` request is a hard error.
- a live endpoint schema mismatch is a hard error or visible empty/error state, not demo data.

Scope:

- `tmp/runners/demo-truth/context/FALLBACK-POLICY.md`

### Z03 - Scenario State Contract Doc

Create `tmp/runners/demo-truth/context/SCENARIO-STATE-CONTRACT.md`.

State contract:

- step states: `pending`, `running`, `completed`, `failed`, `skipped`, `timeout`
- scenario states: `idle`, `preflighting`, `running`, `completed`, `failed`, `skipped`
- a scenario is completed only if every non-skipped step completed
- terminal not connected before timeout means skipped, not fake connected
- unsupported CLI command means skipped or failed with explanation, not a no-op
- every failed/skipped scenario should expose command, cwd, output excerpt, and artifact/log path

Scope:

- `tmp/runners/demo-truth/context/SCENARIO-STATE-CONTRACT.md`

## Group A: Stop Fake Success

Goal: make the app unable to present broken live data as working demo data.

Depends on: Group 0.

### A01 - Harden Base API Fetching

Update the base API hook so it rejects wrong content types and supports DELETE.

Required behavior:

- if request path starts with `/api/` and response content-type is `text/html`, throw an API error
- include response status and URL in thrown errors
- add `delete()` support if not already present
- keep GET/POST call sites type-compatible where possible

Scope:

- `demo/demo-app/src/hooks/useApi.ts`

Verify:

- `cd demo/demo-app && npx tsc --noEmit`

### A02 - Harden `useApiWithFallback`

Rewrite fallback behavior around the Z02 policy.

Required behavior:

- expose `dataMode: 'checking' | 'live' | 'demo'`
- GET uses sample data only when health probe confirms server unreachable
- GET does not fall back when server is live but endpoint fails
- POST/DELETE never return `{}` on failure
- failed mutation callers receive a thrown error

Scope:

- `demo/demo-app/src/hooks/useApiWithFallback.ts`

Also read:

- `tmp/runners/demo-truth/context/FALLBACK-POLICY.md`
- `demo/demo-app/src/hooks/useApi.ts`

Verify:

- `cd demo/demo-app && npx tsc --noEmit`

### A03 - Visible Live/Demo Mode Indicator

Add a small global mode indicator to the shell.

Required behavior:

- shows `LIVE`, `DEMO`, or `CHECKING`
- demo mode copy says sample data is being shown because server is unreachable
- does not cover app controls

Scope:

- `demo/demo-app/src/components/AppShell.tsx`
- related CSS if needed

Verify:

- `cd demo/demo-app && npx tsc --noEmit`

### A04 - Remove Inline Numeric Fallbacks

Remove hardcoded fallback stats from visible dashboard/landing surfaces.

Examples to remove:

- `tasks || 827`
- `agents.length || 5`
- hardcoded cost/pass-rate values displayed as live values

Use real zero, loading, or `-` instead.

Scope:

- `demo/demo-app/src/pages/dashboard/AgentFleet.tsx`
- `demo/demo-app/src/pages/dashboard/CostDashboard.tsx`
- `demo/demo-app/src/pages/Landing.tsx`

Verify:

- `cd demo/demo-app && npx tsc --noEmit`

### A05 - Zero/NaN Display Guards

Fix derived display values after fallback removal.

Required behavior:

- `0 / 0` displays `-`, not `NaN%`
- very small negative costs display `$0.00`, not `$-0.0000`
- `0` displays as `0`, not empty

Scope:

- `demo/demo-app/src/pages/dashboard/CostDashboard.tsx`
- `demo/demo-app/src/pages/Landing.tsx`

Verify:

- `cd demo/demo-app && npx tsc --noEmit`

## Group B: Bench, Share, and API Alignment

Goal: make the bench and share pages call the actual backend and fail visibly when backend state
is missing.

Depends on: A01, A02.

### B01 - Bench Hook Route and Shape Alignment

Fix `useBench.ts` against the canonical backend contract.

Required behavior:

- start: `POST /api/bench/run`
- detail poll: `GET /api/bench/run/:id`
- cancel: `DELETE /api/bench/run/:id`
- list unwraps `{ runs, total, limit, offset }`
- suites unwrap `{ suites }`
- models unwrap `{ models }`
- no fake `demo-${Date.now()}` active run after a failed POST
- active run total comes from backend run/suite data, not only fallback suite task count

Scope:

- `demo/demo-app/src/hooks/useBench.ts`

Also read:

- `tmp/runners/demo-truth/context/API-CONTRACT.md`
- `crates/roko-serve/src/routes/bench.rs`

Verify:

- `cd demo/demo-app && npx tsc --noEmit`

### B02 - Bench Detail Route and Not-Found State

Fix run detail loading.

Required behavior:

- fetch `GET /api/bench/run/:id`
- detect 404 and show "Run not found"
- detect API HTML leak as error via A01
- never stay forever on `Loading run ...`
- handle backend run shape where config may be `overrides` or absent

Scope:

- `demo/demo-app/src/pages/BenchRunDetail.tsx`

Verify:

- `cd demo/demo-app && npx tsc --noEmit`

### B03 - Bench SSE Filtering

The frontend passes `?bench_id=...`; the backend currently emits all bench events.

Choose the smallest safe fix:

- client filters incoming events by `run_id` when present
- if event has no `run_id`, ignore it for a run-specific active panel
- document that backend query filtering is a later server improvement

Scope:

- `demo/demo-app/src/hooks/useBenchSSE.ts`
- `demo/demo-app/src/hooks/useBench.ts` if event handling needs run-id checks

Verify:

- `cd demo/demo-app && npx tsc --noEmit`

### B04 - Bench Backend Index Consistency

Fix the server-side inconsistency found during audit:

- `DELETE /api/bench/run/:id` marks the run file cancelled but list still shows running
- completed index entries can lose `started_at` and show `0`

Required behavior:

- cancel updates the index entry to `cancelled`
- completion update preserves original `started_at`
- list and detail agree on status

Scope:

- `crates/roko-serve/src/routes/bench.rs`
- bench index helper functions if needed

Verify:

- `cargo check -p roko-serve`

### B05 - Share Route Convergence

Fix frontend share route usage.

Required behavior:

- `/share/:token` fetches `/api/shared/:token`, not `/api/share/:token`
- returned backend share URLs are documented if still `/runs/:token`
- if feasible in a small patch, normalize frontend navigation to one public share route

Scope:

- `demo/demo-app/src/pages/Share.tsx`
- optionally `demo/demo-app/src/pages/dashboard/ShareView.tsx` if route handling needs alignment

Also read:

- `crates/roko-serve/src/routes/shared_runs.rs`

Verify:

- `cd demo/demo-app && npx tsc --noEmit`

## Group C: Dashboard and Explorer Live Data Coherence

Goal: make pages render real payload shapes instead of half-live, half-demo state.

Depends on: A01-A05.

### C01 - Explorer Events and Episodes Shape Handling

Fix Explorer's real payload handling.

Required behavior:

- unwrap `/api/statehub/events` from `{ events: [...] }`
- render event envelopes without crashing
- episode expanded view shows key fields first and raw JSON behind a details/toggle section
- no fallback sample events while live server returns an empty list

Scope:

- `demo/demo-app/src/pages/Explorer.tsx`

Verify:

- `cd demo/demo-app && npx tsc --noEmit`

### C02 - Cost Dashboard Dynamic Metrics

Make the cost dashboard render whatever c-factor submetrics the API returns.

Required behavior:

- use `Object.entries(sub_metrics)` rather than a hardcoded metric array
- format `snake_case` keys into readable labels
- show no-data state for absent metrics
- keep gate pass/cost/task displays guarded for zero data

Scope:

- `demo/demo-app/src/pages/dashboard/CostDashboard.tsx`

Verify:

- `cd demo/demo-app && npx tsc --noEmit`

### C03 - Knowledge Graph Stats Consistency

Fix the contradiction where header shows fallback nodes/edges while graph body is live empty.

Required behavior:

- header stats and graph data come from the same source
- live empty entries/edges show an empty state
- demo sample graph appears only in demo mode

Scope:

- `demo/demo-app/src/pages/dashboard/KnowledgeGraph.tsx`
- any local hook/component it uses

Verify:

- `cd demo/demo-app && npx tsc --noEmit`

### C04 - Agent Fleet Live Summary

Make fleet summary derive from the rendered live agents.

Required behavior:

- `TOTAL`, `ACTIVE`, average reputation, tasks done all derive from the current agent list
- no hardcoded 827-style totals
- `null` costs render as `-` or `$0.00` only if semantically zero
- active status uses current payload fields consistently

Scope:

- `demo/demo-app/src/pages/dashboard/AgentFleet.tsx`

Verify:

- `cd demo/demo-app && npx tsc --noEmit`

### C05 - Cascade Router Display Normalization

Make router display match real payload shape.

Required behavior:

- use `total_observations` when present
- show only models with confidence stats in the model table
- separate configured role table from learned observations
- no fake average confidence if there are no observations

Scope:

- `demo/demo-app/src/pages/dashboard/CascadeRouter.tsx`

Verify:

- `cd demo/demo-app && npx tsc --noEmit`

## Group D: Terminal and Demo Scenario Truth

Goal: stop terminal/demo pages from reporting false connection or starting scenarios with missing
handles.

Depends on: A01-A02.

### D01 - Terminal Error State Truth

Fix terminal WebSocket status handling.

Required behavior:

- `ws.onerror` does not set status to `connected`
- expose `error` or `disconnected` state when connection fails
- reconnect behavior can stay, but UI must not show connected until `onopen`

Scope:

- `demo/demo-app/src/hooks/useTerminal.ts`

Verify:

- `cd demo/demo-app && npx tsc --noEmit`

### D02 - Demo Scenario Handle Readiness

Fix the undefined handle crash.

Required behavior:

- filter handles with `h != null`
- require `entries.length === scenario.panes`
- Play button disabled or preflighted until all panes have open WebSockets
- failed readiness shows "waiting for terminals" or "terminal unavailable"
- no `Cannot read properties of undefined (reading 'ws')`

Scope:

- `demo/demo-app/src/pages/Demo.tsx`
- `demo/demo-app/src/hooks/useTerminalSession.ts` only if helper changes are needed

Verify:

- `cd demo/demo-app && npx tsc --noEmit`

### D03 - Builder Workspace and Failure Truth

Make Builder stop running before workspace setup and stop showing building after a failed command.

Required behavior:

- preset/build controls disabled until `setupWorkspace` resolves
- effective workspace path shown somewhere unobtrusive
- if terminal output contains clear failure markers (`workflow halted`, missing key, HTTP 429),
  UI ends in failed state rather than `BUILDING...`
- selected model is either passed through if supported or marked "UI selection not yet wired"

Scope:

- `demo/demo-app/src/pages/Builder.tsx`
- `demo/demo-app/src/hooks/useTerminalSession.ts` if command result needs failure detection

Verify:

- `cd demo/demo-app && npx tsc --noEmit`

### D04 - Demo Scenario Unsupported Commands

Prevent known-broken scenario commands from pretending to run.

Required behavior:

- Providers scenario must not call unsupported `roko run --provider ...`
- either update to a supported command or preflight and mark provider panes skipped with a clear
  message
- Mirage scenario should be visibly placeholder/skipped until real controls exist
- Build tab should either render its prompt bar or link/redirect to `/builder`

Scope:

- `demo/demo-app/src/lib/scenarios.ts`
- `demo/demo-app/src/pages/Demo.tsx`

Verify:

- `cd demo/demo-app && npx tsc --noEmit`

## Group E: Small CLI Truth Fixes

Goal: include only CLI fixes that are genuinely small and directly affect demo credibility.

Depends on: Group 0.

### E01 - Resume Uses Canonical Plans Directory

The top-level `roko resume` path delegates to `plan run` with `workdir.join("plans")`.
Use `roko_cli::plan::plans_dir(&workdir)` so `.roko/plans` works.

Scope:

- `crates/roko-cli/src/main.rs`

Verify:

- `cargo check -p roko-cli`

### E02 - Negative-Zero Cost Formatting

Clamp tiny negative display values to zero at formatting boundaries.

Known likely places:

- `crates/roko-cli/src/commands/util.rs`
- `crates/roko-cli/src/status.rs`
- dashboard/status formatting helpers

Required behavior:

- `$-0.0000` never appears in status output
- do not change stored numeric values unless source is clearly display-only

Scope:

- narrow formatting helper/file found by source inspection

Verify:

- `cargo check -p roko-cli`

### E03 - Unknown Explain Topic Is Not Silent Success

`roko explain "cascade routing"` currently reports unknown topic and exits 0.

Required behavior:

- unknown topic returns nonzero, or
- command returns a structured "not found" status that callers can interpret as failed/skipped
- include suggestion for nearest known topic such as `routing` if cheap

Scope:

- `crates/roko-cli/src/explain.rs`
- command caller in `crates/roko-cli/src/main.rs` if exit code is decided there

Verify:

- `cargo check -p roko-cli`

## Group F: Proof Harness

Goal: keep the runner honest and produce reproducible artifacts.

Depends on: Groups B, C, D, E.

### F01 - API Smoke Script

Add a safe API smoke script.

Required behavior:

- curls every Z01 endpoint that is safe to call
- validates status and JSON content-type
- allows expected 404 for a fake share token
- fails if `/api/*` returns HTML
- does not start model/agent work

Scope:

- `demo/demo-app/scripts/api-smoke.sh`

Verify:

- `bash demo/demo-app/scripts/api-smoke.sh`

### F02 - Bench Start/Cancel Contract Proof

Add a bench proof script that is safe by default.

Required behavior:

- starts a smoke bench run only when `ROKO_DEMO_ALLOW_LIVE_BENCH=1`
- otherwise performs route/shape checks without launching work
- if live run is allowed, immediately polls detail once and then cancels
- verifies list/detail agree on cancelled status after cancel

Do not wait for full benchmark completion by default; that can spend money or hit rate limits.

Scope:

- `demo/demo-app/scripts/bench-proof.sh`

Verify:

- `bash demo/demo-app/scripts/bench-proof.sh`

### F03 - Browser Route Smoke Script

Add a Playwright route smoke script or document an existing one.

Required behavior:

- visits the major routes from the audit
- records console errors, failed requests, screenshots
- fails on unexpected API 4xx/5xx or HTML API responses
- writes artifacts under `/tmp/roko-demo-audit-*`

Scope:

- `demo/demo-app/scripts/route-smoke.mjs`
- package script only if the repo already uses that pattern

Verify:

- `cd demo/demo-app && node scripts/route-smoke.mjs`

### F04 - Demo App Build and Type Gate

Add or document final verification:

```text
cd demo/demo-app
npm run build
npx tsc --noEmit
```

If `npm run build` already runs `tsc -b`, do not duplicate in package scripts; just document final
runner verification.

Scope:

- no code required unless a script entry is useful

Verify:

- `cd demo/demo-app && npm run build`

## Primary Batch Summary

| Group | Batches | Main write scope |
|---|---:|---|
| 0: Contracts | 3 | `tmp/runners/demo-truth/context/*` |
| A: Stop fake success | 5 | API hooks, shell indicator, dashboard fallback guards |
| B: Bench/share/API | 5 | bench hooks/pages, share page, bench server index |
| C: Dashboard/explorer | 5 | Explorer and dashboard pages |
| D: Terminal/demo | 4 | terminal hook, Demo, scenarios, Builder |
| E: Small CLI truth | 3 | resume path, cost display, explain exit |
| F: Proof harness | 4 | scripts and final checks |
| Total | 29 | 26 implementation/proof + 3 contracts |

## Suggested Execution Waves

Wave 1:

- Z01, Z02, Z03

Wave 2:

- A01, A02

Wave 3:

- A03, A04, A05

Wave 4:

- B01, B04, B05, C01

Wave 5:

- B02, B03, C02, C03

Wave 6:

- C04, C05, D01, E01

Wave 7:

- D02, D03, E02, E03

Wave 8:

- D04

Wave 9:

- F01, F02, F03, F04

This wave plan avoids the worst same-file conflicts. It is more useful than "run all of B/C/D in
parallel" because `useBench.ts`, `Demo.tsx`, and dashboard files otherwise become conflict hot
spots.

## Moved Out Of Primary Scope

These were in earlier plans but should not be primary batches here.

| Item | Why moved out |
|---|---|
| Shell gate wiring as "one line" | `GateService` receives only gate names in `roko_core::foundation::GateConfig`; running configured shell program/args needs a wider contract or adapter. |
| Stub gates skipped field | Adding a real skipped state crosses core gate verdict types and consumers. Do later with telemetry schema work. |
| `cost_usd: Option` migration | Cost fields are widespread across core, CLI, TUI, learning, and agent crates. Too broad for demo-truth. |
| Claude CLI usage parsing | Important, but belongs in a telemetry-truth runner unless a tiny isolated patch is proven first. |
| One cost event per attempt | Touches `orchestrate.rs` efficiency semantics; not needed to stop UI fake success. |
| Unknown-model cleanup | Crosses model resolution/event logging; useful but not demo-critical. |
| `roko init` schema v2 | Still important, but source/template impact needs separate inspection. |
| Provider health AppState wiring | Server-side design and background probing policy needed. |
| Full `--model` contract | Architectural model-selection runner. |
| ChatAgentSession / dispatch rewrite | Mori parity runner, not demo-truth. |

## Stretch Batches

Only add these if primary batches finish cleanly and there is no merge pressure.

### S01 - `roko init` Schema Preflight Message

Smallest useful version: when config is v1/missing `[providers]`, print a clear migration/preflight
message in commands used by the demo. Do not rewrite init templates unless source inspection proves
it is contained.

### S02 - Plan Generator Role Field Prompt

Add an explicit `role = "implementer"` requirement to generated `tasks.toml` instructions. Avoid
model alias normalization here unless a local helper already exists.

### S03 - Server `/api/*` JSON 404

If not already handled by A01 content-type guard, fix server routing so unmatched `/api/*` returns
JSON 404 instead of the SPA shell.

### S04 - Safe Provider Preflight Display

Add a read-only provider preflight display to the demo. Do not start provider runs.

## Acceptance Criteria For This Runner

The runner is done when:

- `npm run build` passes in `demo/demo-app`.
- API smoke script reports JSON for API routes and no SPA HTML leaks.
- `/bench` starts no fake active run after a failed POST.
- `/bench` can start/cancel a real run when live bench proof is explicitly enabled.
- `/bench/run/:id` loads real ids and shows not-found for missing ids.
- `/share/:token` uses `/api/shared/:token`.
- `/dashboard` never shows `NaN%` or mixed fallback/live headline numbers.
- `/dashboard/knowledge` header and graph agree.
- `/explorer` events tab handles real event envelopes.
- `/terminal` does not report connected after WebSocket error.
- `/demo` does not throw undefined-handle errors.
- known unsupported demo scenarios are skipped/failed with visible reasons.
- `roko resume` uses the canonical plans dir.
- status output no longer prints `$-0.0000`.

## Bottom Line

This should be a demo-truth runner, not a mini Mori-parity runner. The biggest win is making
false success impossible. Once this lands, the remaining failures will be visible and therefore
much easier to prioritize into the next runners: chat parity, model-selection truth, telemetry
truth, grounded planning, and security hardening.
