# Current Demo App Issue Tracker

Created: 2026-04-29  
Last updated: 2026-04-29  
Scope: `demo/demo-app` plus backend/API surfaces it depends on.

## Current Baseline

- Frontend build: `npm run build` passes from `demo/demo-app`.
- Current Vite warning: only the lazy `vendor-three` chunk is just over the default 500 kB warning threshold.
- Initial JS chunk after route splitting: `260.61 kB` minified.
- Terminal implementation is now split into a small route chunk plus a lazy `vendor-xterm` chunk.
- Live-data rule: demo-app should not synthesize operational data on API failure. Missing backend support should be added to roko instead.
- Source scan checked: removed the known synthetic dashboard metric defaults. Remaining `fallback` mentions are Suspense/CSS comments or terminal renderer terminology, not data substitution.

## Fixed In This Pass

- Removed redundant Demo carousel scenarios: `Self-Hosting` and `Build`.
- Replaced `useApiWithFallback` with `useLiveApi`; API failures now throw instead of returning synthetic data or empty write success.
- Deleted frontend synthetic data modules: `demo-data.ts`, `bench-demo-data.ts`, and `model-catalog.ts`.
- Removed the static Builder model catalog; Builder now uses live `/api/config` model data.
- Removed PRD pipeline prebuilt artifact state; examples remain only as inputs for live runs.
- Removed CostRace static rows; it now reads `/api/bench/cost-summary` and bench SSE events.
- Added roko backend support for `GET /api/bench/cost-summary`, aggregating real persisted bench result cost, token, and task totals by model.
- Aligned bench matrix and Pareto API shapes across frontend/backend: matrix start returns both `id` and `matrix_id`, lane summaries include status counts, and Pareto responses expose `points` plus backend-compatible totals.
- Removed unsupported SWE-bench UI wiring from the demo until roko exposes real SWE-bench routes.
- Removed the fixed integrity hash; `IntegrityView` hashes the latest live `/api/episodes` row.
- Centralized config model identity handling so selectors use config keys consistently.
- Centralized demo workspace creation through the live workspace API before scenario execution; scenarios now receive an actual server-created workspace path.
- Improved API liveness polling, terminal readiness handling, config bootstrap URL handling, and SSE cleanup guards.
- Added a shared flat icon system and wired it through navigation, panes, mosaic metrics, dashboard tabs, and Explorer labels/cards.
- Raised small label fonts across the demo app for readability.
- Added route-level code splitting for major pages and dashboard subroutes.
- Split xterm and Three.js into explicit vendor chunks, and stopped the global ambient particle field from importing Three.js.
- Removed synthetic dashboard defaults from Cost Dashboard, Cascade Router, Knowledge Entries, Knowledge Graph, Dreams, Agent Fleet, and Explorer provider identity rendering.
- Removed the known hook dependency suppressions in Demo session/ref setup, BenchCompare auto-selection, and Cost Dashboard counter animation.

## Open Issues

### P0. Terminal execution still depends on shell marker injection

Evidence:
- `useTerminal.ts` and `useTerminalSession.ts` detect completion by injecting marker output into the shell stream.

Impact:
- Quoting, alternate shells, long-running processes, or commands that take over the terminal can break completion detection.

Fix direction:
- Move command execution to a backend session API that emits lifecycle events with exit status, stdout, and stderr.

### P1. Hook dependency suppressions still need a broad scan

Evidence:
- The known suppressions in Demo, BenchCompare, and CostDashboard were removed.
- A repo-wide pass is still needed for less obvious stale-closure cases outside those files.

Impact:
- Scenario switches, model changes, and terminal handle changes can drift if closures capture stale state.

Fix direction:
- Remove suppressions where possible; isolate one-time invariants into refs with explicit comments.

### P1. Remaining oversized async vendor chunk

Evidence:
- Vite production build now emits an initial JS chunk of `260.61 kB`.
- Vite still warns about lazy `vendor-three` at `501.61 kB` minified.

Impact:
- First load is materially smaller, and xterm is isolated. Views that use the older Three-backed hero/constellation components still pull a large dependency chunk.

Fix direction:
- Replace or lazy-load the remaining Three-backed `HeroScene` / `WorkflowConstellation` usage.
- Consider removing Three entirely unless a route has a concrete 3D interaction that needs it.

### P1. Terminal page and demo still use separate terminal panes

Evidence:
- `pages/Terminal.tsx`, `components/Terminal/TerminalPane.tsx`, and `pages/Demo.tsx` still maintain separate pane wrappers.

Impact:
- Status, sizing, keyboard behavior, and automation safeguards can diverge between pages.

Fix direction:
- Extract one terminal pane primitive with status/close/automation props and consume it from Demo and Terminal.

### P1. Live/offline status components are still fragmented

Evidence:
- Top nav, Cost Dashboard, Config Widget, and LiveIndicator still render their own status labels/badges.

Impact:
- The app can describe the same server state differently across pages.

Fix direction:
- Add one `ConnectionBadge`/`LiveStatus` primitive and replace page-local status markup.

### P2. Frontend still contains scripted demo orchestration

Evidence:
- `scenarios.ts` still contains scripted stage text and terminal choreography for repeatable demo runs.

Impact:
- The orchestration itself is intentional, but every metric/status shown by those scripts should continue to come from live command/API output.

Fix direction:
- Keep scenario scripts as workflow drivers only. Continue removing any scripted metric that is not parsed from live runtime output.

### P2. Empty states need better operator-facing error detail

Evidence:
- Some views now correctly render empty states when live APIs fail or return no rows.

Impact:
- Live-only behavior is more truthful, but users may need clearer server/offline/error details.

Fix direction:
- Add typed error states per dashboard without adding synthetic replacement data.
