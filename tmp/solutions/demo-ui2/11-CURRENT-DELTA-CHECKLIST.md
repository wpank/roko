# 11. Current Delta Checklist

What the current `demo/demo-app/` codebase needs to match the spec. Every item has: context (why), acceptance criteria (how to verify), files to modify, files to delete, and spec reference.

Generated from live codebase audit on 2026-04-29. Enriched with hook-to-consumer mapping and inline style analysis.

---

## Status Key

- `[ ]` — Not started
- `[~]` — Partially done
- `[x]` — Complete

---

## A. Architecture (from 02-ARCHITECTURE)

### A1. DataHub — Replace Scattered State

**Context:** The app has 14 active hooks totaling 2,230 LOC, 3 context providers (`EventStreamProvider`, `WorkspaceProvider`, `RokoConfigProvider`), and 53 `useState` calls across hooks alone. Data flows through 5 different fetching patterns (raw fetch, useApi wrappers, SSE managers, WS connections, health polling). There are 2 competing server-health singletons with module-level mutable state. DataHub collapses all of this into one Zustand store with typed actions.

**Spec reference:** `02-ARCHITECTURE.md` §2 (DataHub), `03-REALTIME-DATA.md` §2 (transport)

---

#### A1.1 `[x]` Create Zustand DataHub store

**Current state:** 14 hooks × 53 `useState` calls + 3 context providers.

Provider hierarchy in `main.tsx`:
```
BrowserRouter > EventStreamProvider > WorkspaceProvider > ErrorBoundary > Suspense > Routes > AppShell
```
`RokoConfigProvider` is exported from `useRokoConfig.ts` but not wrapped in the tree.

**Acceptance criteria:**
- Single `DataHub.ts` file with domain slices (agents, bench, config, knowledge, pipeline, terminal, workspace)
- `npm run build` passes
- Grep `useState` in hooks/ returns 0 results (all state in DataHub)
- Grep `createContext` in hooks/ returns 0 results

**Files to create:** `src/data/DataHub.ts`
**Files to modify:** Every consumer of current hooks (34 files import `useLiveApi` alone)
**Files to delete after migration:**
- `src/hooks/useEventStream.ts` (151L) — singleton manager → DataHub transport
- `src/hooks/useWorkspace.ts` (109L) — context provider → DataHub slice
- `src/hooks/useRokoConfig.ts` (175L) — context provider → DataHub slice
- `src/contexts/EventStreamContext.tsx` — provider wrapper → DataHub
- `src/contexts/WorkspaceContext.tsx` — re-export → DataHub

---

#### A1.2 `[x]` Create typed REST client

**Current state:** Two competing API wrappers:
- `useApi.ts` (33L): bare `fetch()` wrapper, 6 importers
- `useLiveApi.ts` (103L): adds health polling (5s `GET /health`), module-level `_serverLive`, `_healthProbeInFlight`, `_healthListeners` — **34 importers** (most-used hook)
- `useApiWithFallback.ts` (148L): git-deleted but physically on disk, adds demo-data fallback, module-level `_serverLive`, `_probePromise`, `_seedCount`, `_nonSeedCount` — **still imported by AppShell**

**17 distinct API endpoints** called across hooks (see endpoint table in §A1 notes below).

**Acceptance criteria:**
- Single `RokoApi` class with typed methods per endpoint
- One health probe, one `isLive` signal
- `grep -r '_serverLive\|_healthProbe\|_probePromise' src/hooks/` returns 0
- All 17 endpoints covered with proper TypeScript return types

**Files to create:** `src/data/transport/api.ts`
**Files to delete:**
- `src/hooks/useApi.ts` (33L)
- `src/hooks/useLiveApi.ts` (103L)
- `src/hooks/useApiWithFallback.ts` (148L) — already git-deleted, remove from disk

**API endpoints to type:**

| Endpoint | Method | Current Consumer |
|---|---|---|
| `/health` | GET | useLiveApi, useApiWithFallback, useServerHealth |
| `/api/config` | GET, PUT | useRokoConfig |
| `/api/workspaces/default` | GET | useWorkspace |
| `/api/workspaces` | POST | useWorkspace |
| `/api/workspaces/{id}` | DELETE | useWorkspace |
| `/api/bench/suites` | GET | useBench |
| `/api/bench/models` | GET | useBench, useMatrixBench |
| `/api/bench/runs` | GET, POST | useBench |
| `/api/bench/runs/{id}` | GET | useBench |
| `/api/bench/runs/{id}/cancel` | POST | useBench |
| `/api/bench/pareto` | GET | useBench |
| `/api/bench/export/{id}` | GET | useBench |
| `/api/bench/matrix` | POST | useMatrixBench |

---

#### A1.3 `[x]` Create SSE adapter

**Current state:** 3 independent SSE implementations:
- `useEventStream.ts` (151L): factory-pattern `createEventStreamManager()`, connects to `/api/events`, used via `EventStreamContext` (4 importers)
- `useBenchSSE.ts` (88L): connects to `/api/bench/events`, 6 importers (useBench, CostRace, useMatrixBench)
- Dead `useSSE.ts`: git-deleted, was a third SSE wrapper

Each manages its own reconnect logic, event parsing, and connection state independently.

**Acceptance criteria:**
- Single `SseAdapter` class with subscription-based dispatch
- One SSE connection (multiplexed by event type)
- Auto-reconnect with exponential backoff and cursor replay
- `grep -r 'new EventSource' src/` returns 1 result (in SseAdapter)

**Files to create:** `src/data/transport/sse.ts`
**Files to delete:**
- `src/hooks/useEventStream.ts` (151L)
- `src/hooks/useBenchSSE.ts` (88L)
- `src/contexts/EventStreamContext.tsx`

---

#### A1.4 `[x]` Create WebSocket adapter

**Current state:** 2 independent WS implementations:
- `useTerminal.ts` (284L): PTY WS to `ws://SERVE_URL/ws/terminal/{sessionId}`, xterm.js integration, auto-reconnect, command sequencing (`execSeq` module-level counter) — 12 importers
- `useChain.ts` (213L): chain insights WS to `ws://localhost:8545/api/ws`, Phase 2+ feature — 2 importers

**Acceptance criteria:**
- Single `WsAdapter` with subscription model
- Terminal sessions managed by sessionId key
- Chain WS optional, gated on feature flag
- `grep -r 'new WebSocket' src/` returns 1 result (in WsAdapter)

**Files to create:** `src/data/transport/ws.ts`
**Files to modify:** `useTerminal.ts` (keep xterm-specific logic, delegate WS to adapter)
**Files to delete:** `src/hooks/useChain.ts` (213L) — inline into WsAdapter

---

#### A1.5 `[x]` Wire transport → DataHub → components

**Current state:** Events scattered to individual hooks. Each hook manages its own state lifecycle. Provider hierarchy requires components to be mounted in specific tree positions.

**Acceptance criteria:**
- Single `handleServerEvent(event)` function dispatches to DataHub slices
- Remove `EventStreamProvider` and `WorkspaceProvider` from `main.tsx`
- All route components work identically regardless of tree position
- `grep -r 'Provider' src/main.tsx` returns only `BrowserRouter`

**Files to modify:** `src/main.tsx`, `src/App.tsx` or equivalent
**Files to delete:**
- `src/contexts/EventStreamContext.tsx`
- `src/contexts/WorkspaceContext.tsx`

---

#### A1.6 `[x]` Replace 14 hooks with thin selectors

**Current state:** Import frequency by hook:

| Hook | Importers | LOC | useState calls |
|---|---|---|---|
| useLiveApi | 34 | 103 | 2 |
| useCanvasSetup | 32 | 60 | 0 |
| useDebouncedRefetch | 12 | 21 | 0 |
| useTerminal | 12 | 284 | 2 |
| useRokoConfig | 11 | 175 | 6 |
| useWorkspace | 7 | 109 | 2 |
| useBench | 6 | 533 | 19 |
| useBenchSSE | 6 | 88 | 4 |
| useMatrixBench | 6 | 284 | 8 |
| useServerHealth | 5 | 28 | 2 |
| useEventStream | 4 | 151 | 0 |
| useChain | 2 | 213 | 6 |
| useApiWithFallback | 2 | 148 | 2 |
| useApi | 6 | 33 | 0 |

**Acceptance criteria:**
- Each "hook" is 5-15 lines: `const data = useDataHub(s => s.bench.activeRun)`
- `useCanvasSetup` (60L) and `useDebouncedRefetch` (21L) remain as utility hooks (no state, DOM only)
- Total hook LOC drops from 2,230 to ~200
- `useBench.ts` (533L, 19 useState calls) becomes `useBenchSlice` (10L selector)

**Files to create:** `src/data/selectors/*.ts` (thin selector hooks per domain)
**Files to delete:** All 14 hooks except `useCanvasSetup.ts` and `useDebouncedRefetch.ts`

---

#### A1.7 `[x]` Eliminate duplicate server-probe singletons

**Current state:** Two independent health probes racing:
- `useLiveApi.ts:8-10`: `let _serverLive: boolean | null = null; let _healthProbeInFlight: Promise<void> | null = null; let _healthListeners: Set<() => void> = new Set()`
- `useApiWithFallback.ts:7-11`: `let _serverLive: boolean | null = null; let _probePromise: Promise<void> | null = null; let _seedCount = 0; let _nonSeedCount = 0`
- `useServerHealth.ts` (28L): third independent `GET /health` poll (5s default, 5 importers)

**Three hooks independently polling the same `/health` endpoint.**

**Acceptance criteria:**
- One health probe in DataHub, polled once, result shared
- `grep -rn '_serverLive\|_healthProbe\|_probePromise\|_seedCount' src/` returns 0
- `grep -rn 'GET.*health' src/hooks/` returns 0

**Files to delete:**
- `src/hooks/useServerHealth.ts` (28L) — replaced by DataHub health slice
- Module-level state in `useLiveApi.ts` and `useApiWithFallback.ts` (both deleted per A1.2)

---

### A2. Pipeline State Machine

**Context:** There is no unified lifecycle state. The terminal, pipeline panel, and playback bar each track their own state independently. Switching tabs loses all state. Terminal sessions die on navigation. The playback bar is a child of the content area and gets clipped by `overflow: hidden` on ancestors.

**Spec reference:** `02-ARCHITECTURE.md` §8 (Pipeline State Machine)

---

#### A2.1 `[x]` Add `PipelineStage` type to DataHub

**Current state:** No lifecycle state machine. `Demo.tsx` (832L) uses ad-hoc boolean combinations (`isRunning`, `isPaused`, `currentPhase`) to derive UI state.

**Acceptance criteria:**
- `PipelineStage` discriminated union type with 11 states: `idle | selecting | configuring | starting | prd_generating | planning | executing | gate_checking | paused | failed | complete`
- All pipeline UI derives from this single value
- `typeof DataHub.pipeline.stage` is `PipelineStage`

**Files to create:** `src/data/types/pipeline.ts`
**Files to modify:** `Demo.tsx` — remove ad-hoc boolean state tracking

---

#### A2.2 `[x]` Add Activity Strip to AppShell

**Current state:** Playback bar is inside content area `<div>` with `overflow: hidden` on ancestors. Gets clipped during scroll. Not visible across page navigation.

**Acceptance criteria:**
- Activity Strip renders in `AppShell.tsx` chrome layer, outside any `overflow: hidden` container
- Visible on every page during active pipeline
- Shows current `PipelineStage` with visual indicator
- Persists across route changes

**Files to modify:** `src/components/AppShell.tsx`
**Files to create:** `src/components/ActivityStrip.tsx`

---

#### A2.3 `[x]` Add loading states to every transition

**Current state:** 8 transitions have no loading indicator — content jumps from blank to populated. The 7 pages with zero state handling: `Builder.tsx`, `Demo.tsx`, `Share.tsx`, `AgentFleet.tsx`, `DreamsView.tsx`, `IntegrityView.tsx`, `KnowledgeGraph.tsx`.

**Acceptance criteria:**
- Every data-dependent render has exactly one of: content | skeleton | error | empty
- No blank screens during loading (verify with throttled network in DevTools)
- Skeleton shapes match content shapes (not generic spinners)
- `grep -rn 'Skeleton\|DataSurface' src/pages/` shows coverage in every page

**Files to create:** `src/components/DataSurface.tsx` (wrapper component), `src/components/Skeleton.tsx`
**Files to modify:** All 13 pages listed in D1.1-D1.13

---

#### A2.4 `[x]` Add terminal lifecycle states

**Current state:** Terminal shows blank pane during connecting/idle. Garbled characters appear during `resolveRoko()` probe. `useTerminal.ts` tracks `status` but UI doesn't display it visually.

**Acceptance criteria:**
- Terminal header shows one of: `CONNECTING...` | `CONNECTED` | `EXECUTING cmd` | `IDLE`
- Agent name + model + tier displayed in header when attributed
- No garbled characters visible (probe output suppressed)
- `TerminalPane` component accepts `agent?: AgentIdentity` prop

**Files to modify:** `src/components/Terminal.tsx` or `TerminalPane.tsx`
**Spec reference:** `02-ARCHITECTURE.md` §8.5, `06-AGENT-MODEL.md` §4

---

#### A2.5 `[x]` Add state persistence across navigation

**Current state:** Switching tabs kills terminal WS connections, loses all local state. Returning to Orchestrate mid-run shows blank page. No URL encoding of state.

**Acceptance criteria:**
- Pipeline stage persisted in DataHub (survives route changes)
- Terminal WS connections survive React unmount (managed by DataHub, not component lifecycle)
- URL encodes active scenario + stage (e.g., `?scenario=add-logging&stage=executing`)
- Back button restores previous view state

**Files to modify:** `src/data/DataHub.ts`, `src/App.tsx` (route config)

---

### A3. Module-Level Singletons (from 10-UX-PHILOSOPHY §3)

---

#### A3.1 `[x]` Move PlaybackController/TimelineStepper off module scope

**Current state:** `Demo.tsx:42-43` constructs class instances at import time (module scope). These persist across React renders and can hold stale state.

**Acceptance criteria:**
- Class instances created in `useRef()` inside component, or managed by DataHub
- `grep -rn 'new PlaybackController\|new TimelineStepper' src/` returns 0 at module scope

**Files to modify:** `src/pages/Demo.tsx`

---

#### A3.2 `[x]` Consolidate server health singletons

**Duplicate of A1.7 — see that item for full detail.**

---

#### A3.3 `[x]` Fix `useApiWithFallback` → `useLiveApi` migration

**Current state:** `AppShell.tsx` still imports `useApiWithFallback` which is git-deleted. This works because the file is physically on disk despite being staged for deletion.

**Acceptance criteria:**
- `AppShell.tsx` imports `useLiveApi` instead
- `grep -rn 'useApiWithFallback' src/` returns 0
- Build passes after physical file deletion

**Files to modify:** `src/components/AppShell.tsx`
**Files to delete:** `src/hooks/useApiWithFallback.ts` (148L)

---

#### A3.4 `[x]` Remove git-deleted files still on disk

**Current state:** 3 files are git-deleted (`git rm`) but physically present:

| File | Lines | Why it's dead |
|---|---|---|
| `src/hooks/useApiWithFallback.ts` | 148 | Replaced by `useLiveApi`, only AppShell still imports |
| `src/lib/bench-demo-data.ts` | 252 | Demo/seed data for offline bench — was in useApiWithFallback path |
| `src/lib/demo-data.ts` | 522 | Demo/seed data for all pages — same path |

**Acceptance criteria:**
- All 3 files physically deleted from disk
- `npm run build` passes
- `git status` shows no untracked ghost files in these paths

**Files to delete:**
- `src/hooks/useApiWithFallback.ts` (148L)
- `src/lib/bench-demo-data.ts` (252L)
- `src/lib/demo-data.ts` (522L)

---

## B. Design System (from 04-DESIGN-SYSTEM)

### B1. Missing Design Tokens

**Context:** The ROSEDUST v2 spec defines ~35 token groups. `rosedust.css` implements most color and spacing tokens but is missing motion, focus, glow, shadow, and cell tokens. Without these, components inline their own values (283 raw `rgba()` + 330 inline styles), preventing consistency.

**Spec reference:** `04-DESIGN-SYSTEM.md` §1-4

---

#### B1.1 `[x]` Glass backgrounds — MEDIUM

**Missing:** `--bg-glass` (current `--glass-bg` uses different naming), `--bg-glass-hover`, `--bg-glass-active`
**Current usage:** `background: 'rgba(255,255,255,.04)'` appears 11× across dashboard files. `--glass-bg` exists in rosedust.css but naming doesn't match spec.
**Decision:** Rename `--glass-bg` → `--bg-glass` in rosedust.css, or update spec to match `--glass-bg`. Add hover/active variants.
**Acceptance:** `grep -rn 'rgba(255,255,255,.04)' src/pages/dashboard/` returns 0
**Files to modify:** `src/styles/rosedust.css`, then all dashboard `.tsx` files

---

#### B1.2 `[x]` Border tokens — LOW

**Missing:** `--border-active`
**Current:** `--glass-border` and `--glass-2-border` exist. `--border-active` would be used on focused/selected card borders.
**Acceptance:** Token exists in rosedust.css, used on active card borders
**Files to modify:** `src/styles/rosedust.css`

---

#### B1.3 `[x]` Status glow tokens — HIGH

**Missing:** `--glow-active`, `--glow-success`, `--glow-error`, `--glow-ambient`, `--glow-rose`
**Current hardcoded values:**
- `0 0 6px rgba(122,138,120,.6)` (success glow in CostDashboard status dots)
- `0 0 6px rgba(216,168,120,.6)` (warning glow)
- `0 0 6px rgba(204,144,168,.6)` (rose glow)
- `0 0 10px rgba(220,165,189,.5)` (bright rose glow in AgentFleet)
**Acceptance:** `grep -rn 'boxShadow.*rgba' src/pages/dashboard/` returns only dynamic/computed values
**Files to modify:** `src/styles/rosedust.css`, `CostDashboard.tsx`, `AgentFleet.tsx`, `KnowledgeGraph.tsx`

---

#### B1.4 `[x]` Status idle token — LOW

**Missing:** `--status-idle` (the 5 other status colors `--success`, `--warning`, `--rose-bright`, `--dream-bright`, `--bone` exist)
**Acceptance:** Token exists in rosedust.css
**Files to modify:** `src/styles/rosedust.css`

---

#### B1.5 `[x]` Shadow tokens — MEDIUM

**Missing:** `--shadow-sm`, `--shadow-md`, `--shadow-lg`, `--shadow-glow-rose`
**Current:** No shadow tokens. Shadows are inline: `boxShadow: '0 8px 32px rgba(220,165,189,.12)'` (AgentFleet hover)
**Acceptance:** All `boxShadow` strings in dashboard TSX use `var(--shadow-*)` tokens
**Files to modify:** `src/styles/rosedust.css`, dashboard TSX files

---

#### B1.6 `[x]` Motion easing — HIGH (Tier 0)

**Missing:** `--ease-snappy`, `--ease-expo`, `--ease-out`
**Current:** Only `--ease` exists. `cubic-bezier(.22,1,.36,1)` hardcoded in AgentFleet `FLEET_STYLES`. `ease-in-out` used in 10+ keyframe animations.
**Acceptance:** 3 easing tokens in rosedust.css. `grep -rn 'cubic-bezier' src/` returns only rosedust.css
**Files to modify:** `src/styles/rosedust.css`, `AgentFleet.tsx` (FLEET_STYLES), all CSS files with `cubic-bezier`

---

#### B1.7 `[x]` Motion durations — HIGH (Tier 0)

**Missing:** `--duration-instant` (80ms), `--duration-fast` (150ms), `--duration-normal` (220ms), `--duration-slow` (350ms)
**Current:** Has `--duration-snap` (80ms), `--duration-smooth` (180ms), `--duration-reveal` (280ms) — different names and values from spec.
**Decision needed:** Rename existing tokens to match spec naming, or update spec. Either way, align on one set.
**Acceptance:** Single set of duration tokens, all CSS `transition`/`animation` durations reference them
**Files to modify:** `src/styles/rosedust.css`, all component CSS files using duration values

---

#### B1.8 `[x]` Focus ring token — HIGH (Tier 0)

**Missing:** `--focus-ring` (double-ring pattern for `:focus-visible`)
**Current:** Zero focus ring definitions. Interactive elements have no visible focus indicator.
**Acceptance:** `--focus-ring` token in rosedust.css. Applied to `:focus-visible` on all interactive elements (buttons, links, inputs, cards). Visible double-ring outline.
**Files to modify:** `src/styles/rosedust.css`

---

#### B1.9 `[x]` Cell tokens — MEDIUM

**Missing:** `--cell-radius`, `--cell-padding`, `--cell-gap`, `--cell-border`
**Needed by:** Cell system (G1-G7). Can be deferred until Cell components are built.
**Acceptance:** Tokens exist in rosedust.css, used by Cell.tsx
**Files to modify:** `src/styles/rosedust.css`

---

#### B1.10 `[x]` Gap tokens — LOW (naming decision)

**Current:** Uses `--sp-1` through `--sp-10` (4px increments). Spec uses `--gap-xs` through `--gap-2xl`.
**Decision:** Keep `--sp-*` naming (already widely used across codebase). Update spec to document `--sp-*` as the canonical spacing scale.
**Acceptance:** One naming convention across spec and code. No aliased duplicates.
**Files to modify:** `04-DESIGN-SYSTEM.md` (update spec), OR `src/styles/rosedust.css` (rename tokens)

---

### B2. Raw Colors in CSS (283 violations)

**Context:** 283 raw `rgba()` values and 6 hex colors hardcoded in component CSS files. These can't be themed, can't be maintained, and defeat the design system. The dashboard TSX files are even worse with inline color strings.

**Spec reference:** `04-DESIGN-SYSTEM.md` §2 (color tokens)

---

#### B2.1 `[x]` Replace raw `rgba()` in component CSS — 283 occurrences

**Worst files:**
| File | Count | Primary patterns |
|---|---|---|
| `Bench.css` | 70 | Rose/dream colors at custom opacities |
| `PrdPipelinePanel.css` | 35 | Pipeline phase colors |
| `Explorer.css` | 26 | Activity stream colors |
| `Demo.css` | 23 | Scenario/phase colors |
| `Terminal.css` | 15 | Terminal chrome colors |

**Acceptance:**
- `grep -c 'rgba(' src/**/*.css` returns < 30 (only truly dynamic values in keyframes)
- Every replaced value uses a `var(--*)` token from rosedust.css
- Visual diff: before/after screenshots show identical rendering

**Files to modify:** All 26 CSS files containing raw `rgba()`
**Files to modify first:** `src/styles/rosedust.css` — add opacity variants as needed (e.g., `--rose-deep-70`)

---

#### B2.2 `[x]` Add missing opacity variants to token set

**Most common raw values not yet tokenized:**
- `rgba(58,32,48,0.7)` — rose-deep at 70% → `--rose-deep-70`
- `rgba(220,165,189,0.12)` — rose-bright at 12% → `--rose-bright-12`
- `rgba(255,255,255,0.04)` — white at 4% → already `--glass-bg`
- `rgba(255,255,255,0.07)` — white at 7% → already `--glass-border`
- `rgba(194,184,201,0.5)` — lavender text → `--text-canvas-empty`

**Alternative:** Use CSS `color-mix()` instead of opacity variants:
```css
background: color-mix(in srgb, var(--rose-deep) 70%, transparent);
```

**Acceptance:** Consistent approach chosen (opacity variants OR color-mix). All 283 raw values resolved.
**Files to modify:** `src/styles/rosedust.css`

---

#### B2.3 `[x]` Replace hex colors in CSS — 6 occurrences

**Files:** `Explorer.css`, `Terminal.css`, `Settings.css`, `Builder.css`, `Bench.css`
**Acceptance:** `grep -rn '#[0-9a-fA-F]' src/**/*.css` returns 0
**Files to modify:** 5 CSS files listed above

---

#### B2.4 `[x]` Replace hardcoded hex in TSX canvas code — 5 remaining

**Current:** 2 in `CostRace.tsx`, 3 in `MatrixRaceTrack.tsx` — marked with TODO comments.
**Canvas colors need module-level constants, not CSS variables** (canvas API can't read CSS vars without `getComputedStyle`).
**Acceptance:** Canvas colors defined as module-level `CHART_COLORS` const, values match rosedust tokens
**Files to modify:** `src/components/CostRace.tsx`, `src/components/MatrixRaceTrack.tsx`

---

### B3. Typography Violations

**Spec reference:** `04-DESIGN-SYSTEM.md` §3, `10-UX-PHILOSOPHY.md` §3 item 18

---

#### B3.1 `[x]` Fix 8px font in ErrorState.css

**Location:** `src/components/ErrorState.css:52` — `font-size: 8px` on disclosure `<details>` triangle.
**Spec minimum:** 10px for canvas labels, 11px for UI labels, 12px for table text, 13px for body.
**Acceptance:** Font size ≥ 10px. Verify visually.
**Files to modify:** `src/components/ErrorState.css`

---

#### B3.2 `[x]` Audit 10px usages — 3 files

**Files:** `Badge.css`, `GateBar.css`, `Tabs.css`
**Spec:** 10px allowed only for canvas labels (drawn via `ctx.font`), not for DOM text.
**Acceptance:** DOM `font-size: 10px` changed to `11px` minimum. Canvas `ctx.font = '10px ...'` remains acceptable.
**Files to modify:** 3 CSS files

---

#### B3.3 `[x]` Audit 11px usages — 13 occurrences in 5 files

**Spec minimum for labels is 12px.** 11px is marginal — evaluate each case.
**Acceptance:** Review each; any body text at 11px bumped to 12px. Label text at 11px acceptable if readability is fine.
**Files to modify:** 5 CSS files with 11px occurrences

---

### B4. Animation System

**Context:** The animation system is fragmented: 55 keyframes across 23 CSS files, 73% are component-local, with 7 duplicate "fade in" and 6 duplicate "pulse" variants. Zero `prefers-reduced-motion` support. 30 `transition: all` violations. No Motion library installed.

**Spec reference:** `04-DESIGN-SYSTEM.md` §5 (motion system), `02-ARCHITECTURE.md` §4.6 (animation)

---

#### B4.1 `[x]` Eliminate `transition: all` — HIGH (Tier 0)

**30 occurrences in 10 files** including `rosedust.css` itself.
**Why bad:** `transition: all` transitions every CSS property including `width`, `height`, `z-index` — causes jank and unexpected animations. Spec explicitly prohibits.
**Acceptance:** `grep -rn 'transition:.*all\|transition: all' src/` returns 0. Each replaced with specific properties: `transition: opacity 150ms var(--ease-snappy), transform 150ms var(--ease-snappy)`.
**Files to modify:** `rosedust.css` + 9 component CSS files

---

#### B4.2 `[x]` Consolidate duplicate keyframes — MEDIUM

**Current:** 40 of 55 keyframes (73%) are component-local. At least 7 semantically identical "fade in" animations and 6 "pulse" animations with trivially different parameters.
**Acceptance:** Shared keyframes in `rosedust.css` or `motion.css`. Component CSS uses `animation-name` referencing shared keyframes. ≤ 20 total unique keyframes.
**Files to modify:** `src/styles/rosedust.css` (add shared keyframes), 23 component CSS files (remove duplicates)

---

#### B4.3 `[x]` Add missing spec keyframes — MEDIUM

**Missing:** `value-flash` (highlight on value change), `line-draw` (SVG path animation), `error-flash` (rose flash on failure).
**Acceptance:** 3 keyframes in rosedust.css, used by at least 1 component each
**Files to modify:** `src/styles/rosedust.css`

---

#### B4.4 `[x]` Add `prefers-reduced-motion` — HIGH (Tier 0)

**Complete gap.** `grep -rn 'prefers-reduced-motion' src/` returns **0 results**.
**Acceptance:**
```css
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
```
Added to rosedust.css. Color transitions (semantic, not decorative) can be excepted.
**Files to modify:** `src/styles/rosedust.css`

---

#### B4.5 `[x]` Add `backdrop-filter: saturate(180%)` — LOW

**Current:** All backdrop-filter uses only `blur()`. Spec requires `blur(12px) saturate(180%)` for glass surfaces.
**Acceptance:** `grep -rn 'backdrop-filter' src/` shows `blur(12px) saturate(180%)` everywhere
**Files to modify:** All CSS files with `backdrop-filter`

---

#### B4.6 `[x]` Install Motion library — MEDIUM

**Not installed.** Spec requires `motion` (framer-motion successor) for: spring animations, shared element transitions (cell expand/collapse), AnimatePresence (mount/unmount animations), layout animations.
**Acceptance:** `motion` in `package.json` dependencies. At least one component uses `<motion.div>`.
**Files to modify:** `package.json`

---

#### B4.7 `[x]` Create `motion/tokens.ts` — MEDIUM

**Spring configs, duration constants, stagger delays, preset variants** (`fadeUp`, `scaleIn`, `slideRight`).
**Acceptance:** File exists with typed animation presets. Used by ≥ 3 components.
**Files to create:** `src/design/motion-tokens.ts`

---

## C. Agent Model (from 06-AGENT-MODEL)

**Context:** Agents are currently represented as name strings with no visual identity, no attribution in terminals/logs, no topology visualization using avatars. The Spectre system defines procedural dot-cloud identities deterministic from `hash(name:role)` with 8 archetypes, 7 role palettes, 4 size variants, and eye glyphs.

**Spec reference:** `06-AGENT-MODEL.md` §5-13

---

#### C1 `[x]` Create `AgentIdentity` type

**Current:** Agents are bare strings (`agent.name`). No structured type.
**Target type:**
```ts
type AgentIdentity = {
  name: string; role: AgentRole; domain: string;
  model: string; tier: number;
  spectre: SpectreIdentity;
}
```
**Acceptance:** Type exported from `src/data/types/agent.ts`. Used by terminal headers, fleet cards, topology graph.
**Files to create:** `src/data/types/agent.ts`
**Spec reference:** `06-AGENT-MODEL.md` §5.1

---

#### C2 `[x]` Create SpectreAvatar component

**Current:** No agent avatars anywhere. Agent names shown as text.
**Target:** Procedural dot-cloud canvas rendering. 16/32/48/64px variants. Role-based color palettes (7 roles). 8 body archetypes. Eye glyph per role (◈ implementer, ◉ researcher, ◎ verifier, etc.).
**Acceptance:** `<SpectreAvatar agent={identity} size={32} />` renders deterministic avatar. Same agent always produces same visual.
**Files to create:** `src/components/SpectreAvatar.tsx`
**Spec reference:** `06-AGENT-MODEL.md` §5.2-5.4

---

#### C3 `[x]` Add agent attribution to terminal headers

**Current:** Terminals show raw shell output. No agent name, model, tier, or status.
**Target:** Header bar: `[Spectre glyph] agent-name · claude-sonnet-4-20250514 · tier-2 · ● EXECUTING`
**Acceptance:** Terminal header shows all 4 fields. Spectre avatar renders at 16px inline.
**Files to modify:** `src/components/Terminal.tsx` or `TerminalPane.tsx`
**Spec reference:** `06-AGENT-MODEL.md` §6

---

#### C4 `[x]` Add multi-agent terminal split

**Current:** Fixed 2-pane layout regardless of agent count.
**Target:** Auto-adapt: 1→full width, 2→side-by-side, 3-4→grid, 5+→scrollable row.
**Acceptance:** Layout responds to agent count. Each pane has attributed header.
**Files to modify:** `src/pages/Demo.tsx` or Orchestrate equivalent
**Spec reference:** `06-AGENT-MODEL.md` §7

---

#### C5 `[x]` Add agent-attributed event log

**Current:** `CommandLog` shows timestamped text with no agent attribution.
**Target:** Each log line: `[Spectre 16px] agent-name [EVENT_TYPE badge] message text`
**Acceptance:** Log entries show agent identity. Event type badges color-coded.
**Files to modify:** `src/components/CommandLog.tsx`
**Spec reference:** `06-AGENT-MODEL.md` §8

---

#### C6 `[x]` Add agent topology graph with Spectre nodes

**Current:** `AgentFleet.tsx` has topology canvas with basic circle nodes. No Spectre rendering.
**Target:** Spectre avatars at graph nodes. Breathing animation on active agents. Status rings (green=healthy, amber=degraded, rose=failing). Edge animations for communication.
**Acceptance:** Topology nodes render SpectreAvatar. Active agents pulse.
**Files to modify:** `src/pages/dashboard/AgentFleet.tsx`
**Spec reference:** `06-AGENT-MODEL.md` §9

---

#### C7 `[x]` Add knowledge transfer visualization

**Current:** Not implemented.
**Target:** Particle animation along graph edges when agents share knowledge (neuro store writes).
**Acceptance:** Visual particles flow along edges during knowledge transfer events.
**Files to modify:** `src/pages/dashboard/AgentFleet.tsx` topology canvas
**Spec reference:** `06-AGENT-MODEL.md` §10

---

#### C8 `[x]` Add reusable AgentCard component

**Current:** Agent display is ad-hoc per page. `AgentFleet` builds cards inline. `Demo.tsx` shows agent names as strings.
**Target:** 4 variants: `inline` (16px avatar + name), `badge` (avatar + name + model), `card` (full stats), `hero` (large with topology position).
**Acceptance:** `<AgentCard agent={identity} variant="card" />` renders consistently. Used in ≥ 3 pages.
**Files to create:** `src/components/AgentCard.tsx`
**Spec reference:** `06-AGENT-MODEL.md` §12

---

## D. UX & State Management (from 10-UX-PHILOSOPHY)

### D1. Loading / Empty / Error States

**Context:** 7 pages have zero state handling. 6 pages have partial coverage. 0 pages have complete coverage. This violates UX principle 1.4 (Visual Feedback Everywhere) and anti-pattern AP-08 (Loading State Theater). Every data-dependent surface must show exactly one of: content, skeleton, error, or empty — never blank.

**Spec reference:** `10-UX-PHILOSOPHY.md` §1.4 + §2 AP-08, `02-ARCHITECTURE.md` §8.3-8.7

---

| # | Page | loading | empty | error | Acceptance Criteria |
|---|------|---------|-------|-------|-------------------|
| D1.1 | `[x]` Builder.tsx | NONE | NONE | NONE | Wrap in `<DataSurface>`. Show skeleton for config load. Empty state: "Select a scenario or type a request." Error state: show error + retry. |
| D1.2 | `[x]` Demo.tsx | NONE | NONE | NONE | Pipeline state machine handles all states. `PipelineStage.idle` shows scenario cards. Loading shows skeleton. Error shows inline recovery. |
| D1.3 | `[x]` Share.tsx | NONE | NONE | NONE | Loading: skeleton card. Error: "Receipt not found — check the URL." Empty: impossible (URL-driven). |
| D1.4 | `[x]` AgentFleet.tsx | NONE | NONE | NONE | Loading: card grid skeleton (4 placeholder cards). Empty: "No agents running. Start a scenario in Orchestrate." Error: inline retry. |
| D1.5 | `[x]` DreamsView.tsx | NONE | NONE | NONE | Loading: timeline skeleton. Empty: "No dream cycles yet. Run `roko knowledge dream run`." Error: inline retry. |
| D1.6 | `[x]` IntegrityView.tsx | NONE | NONE | NONE | Loading: table skeleton. Empty: "No custody events. Integrity tracking activates during plan execution." Error: inline retry. |
| D1.7 | `[x]` KnowledgeGraph.tsx | NONE | NONE | NONE | Loading: canvas placeholder with shimmer. Empty: "No knowledge entries. Run a scenario to populate the graph." Error: inline retry. |
| D1.8 | `[x]` Bench.tsx | NONE | 7 empty | 4 error | Add loading skeletons for all 3 tabs. |
| D1.9 | `[x]` BenchRunDetail.tsx | NONE | 5 empty | 3 error | Add loading skeleton for run header + results table. |
| D1.10 | `[x]` Explorer.tsx | NONE | 3 empty | NONE | Add loading skeleton + error wrapper. |
| D1.11 | `[x]` CascadeRouter.tsx | 3 loading | NONE | NONE | Add empty state: "No routing decisions yet." Add error wrapper. |
| D1.12 | `[x]` KnowledgeEntries.tsx | 3 loading | NONE | NONE | Add empty state: "No knowledge entries." Add error wrapper. |
| D1.13 | `[x]` CostDashboard.tsx | 1 loading | NONE | NONE | Add empty state for all sections. Add error wrapper. |

**Files to create:** `src/components/DataSurface.tsx`, `src/components/Skeleton.tsx`, `src/components/EmptyState.tsx`
**Files to modify:** All 13 page files listed above

---

### D2. Inline Style Reduction

**Context:** ~330 inline styles across the app, concentrated in dashboard pages. The 5 worst dashboard pages have **zero** corresponding `.css` files — everything is inline. Many share identical patterns (e.g., `thStyle`/`tdStyle` objects in both `CascadeRouter.tsx` and `KnowledgeEntries.tsx`).

Common extractable patterns across all 5 dashboard files:

| Pattern | Occurrences | Token/Class Replacement |
|---|---|---|
| `fontFamily: 'var(--mono)'` | 36× | `.mono` utility class |
| `fontSize: 13` | 15× | Use `--text-sm` (14px) |
| `display: 'flex'` + gap | 25× | Layout CSS class |
| `background: 'rgba(255,255,255,.04)'` | 11× | `var(--glass-bg)` |
| `border: '1px solid rgba(255,255,255,.07)'` | 8× | `var(--glass-border)` |
| `letterSpacing: '.08em'` + `textTransform: 'uppercase'` | 8× | `.label-caps` class |
| `padding: '6px 10px'` (th/td cells) | 12× | `.table-cell` class |
| `borderRadius: 2` or `4` or `8` | 10× | `var(--radius-sm/md/lg)` |

**Spec reference:** `10-UX-PHILOSOPHY.md` §3 item 10, `04-DESIGN-SYSTEM.md` §2

---

| # | File | Inline Styles | Has .css | Shared Patterns | Target |
|---|------|--------------|----------|----------------|--------|
| D2.1 | `[x]` CostDashboard.tsx | 38 | Yes | `STATUS_DOT_STYLES` const, `METRICS` color array, 14× `fontFamily: mono` | Create `CostDashboard.css`. Extract `status-dot`, `metric-bar`, `activity-block`, `provider-cell` classes. Target < 10 inline (dynamic only). |
| D2.2 | `[x]` KnowledgeGraph.tsx | 33 | Yes | Canvas overlay positioning, domain legend dots, HUD labels | Create `KnowledgeGraph.css`. Extract `.hud-overlay`, `.domain-legend`, `.domain-dot` classes. Canvas colors → module constants. Target < 8. |
| D2.3 | `[x]` CascadeRouter.tsx | 22+2 consts | Yes | `pageStyle`, `thStyle`, `tdStyle` module consts (identical to KnowledgeEntries) | Create `CascadeRouter.css`. Extract shared `table.css` with `.table-header`, `.table-cell` classes. Target < 8. |
| D2.4 | `[x]` KnowledgeEntries.tsx | 19+2 consts | Yes | `pageStyle`, `thStyle`, `tdStyle` module consts (identical to CascadeRouter) | Create `KnowledgeEntries.css`. Share `table.css` with CascadeRouter. Target < 8. |
| D2.5 | `[x]` AgentFleet.tsx | 24+1 const | Yes | `FLEET_STYLES` template literal with keyframes + hover. 5× `fontFamily: mono` | Create `AgentFleet.css`. Move `FLEET_STYLES` keyframes + hover to CSS. Extract `.agent-card`, `.stat-pill`, `.domain-tag` classes. Target < 8. |
| D2.6 | `[x]` DreamPhaseViz.tsx | 23 | No | Canvas positioning, phase labels | Create `DreamPhaseViz.css`. Target < 8. |
| D2.7 | `[x]` Bench.tsx | 21 | Yes (1,789L!) | Some inline despite huge CSS file | Move remaining inline to `Bench.css`. Target < 8. |
| D2.8 | `[x]` DreamsView.tsx | 19 | No | Timeline markers, dream entries | Create `DreamsView.css`. Target < 8. |
| D2.9 | `[x]` BenchRunDetail.tsx | 18 | No (uses Bench.css) | Result table, gate badges | Create `BenchRunDetail.css` or extend `Bench.css`. Target < 8. |
| D2.10 | `[x]` IntegrityView.tsx | 13 | No | Event timeline, custody badges | Create `IntegrityView.css`. Target < 5. |

**Shared extractable CSS:**
- **`src/styles/table.css`** — `.table-header`, `.table-cell`, `.table-row-hover` (shared by CascadeRouter + KnowledgeEntries)
- **`.mono` utility class** in rosedust.css — replaces 36× `fontFamily: 'var(--mono)'` inline
- **`.label-caps` utility class** in rosedust.css — replaces 8× `letterSpacing + textTransform` inline

**Files to create:** 8 new CSS files + 1 shared `table.css`
**Files to modify:** 10 TSX files, `rosedust.css` (add utility classes)

---

### D3. Monolithic File Splits

**Context:** 7 files exceed the 500-line target. The worst is `scenarios.ts` at 1,900 lines — a single file containing all scenario definitions, step sequences, and runner logic.

**Spec reference:** `10-UX-PHILOSOPHY.md` §5.2 ("Do Fewer Things, Perfectly")

---

| # | File | Lines | What to extract | Target |
|---|------|-------|----------------|--------|
| D3.1 | `[x]` `lib/scenarios.ts` | 1,900 | Split into `scenario-registry.ts` (type + registry, ~50L) + per-scenario runner files (`add-logging.ts`, `fix-bug.ts`, `implement-feature.ts` — ~200L each) | Each file < 300L |
| D3.2 | `[x]` `pages/Explorer.tsx` | 861 | Extract `ActivityStream.tsx` (SSE-driven event list), `AgentStatusPanel.tsx`, `SystemMetrics.tsx` as sub-components | Main file < 300L |
| D3.3 | `[x]` `pages/Demo.tsx` | 832 | Extract `ScenarioSelector.tsx`, `PhaseRail.tsx`, `PlaybackControls.tsx`. Pipeline state machine goes to DataHub. | Main file < 250L |
| D3.4 | `[x]` `pages/Bench.css` | 1,789 | Split per-tab: `BenchRun.css`, `BenchMatrix.css`, `BenchHistory.css`. Or convert to CSS modules. | Each file < 500L |
| D3.5 | `[x]` `styles/rosedust.css` | 829 | Split: `tokens.css` (custom properties, ~200L) + `global.css` (resets, utilities, ~200L) + `keyframes.css` (shared animations, ~100L) | Each file < 300L |
| D3.6 | `[x]` `hooks/useBench.ts` | 533 | After DataHub migration (A1.1), this becomes a 10L selector. No split needed — just migrate. | < 15L via A1.6 |
| D3.7 | `[x]` `components/HeroScene.tsx` | 512 | Extract Three.js scene setup (`createScene()`, `createLights()`, `createGeometry()`) to `hero-scene-setup.ts`. Keep React wrapper thin. | Main file < 200L |

**Files to create:** Per-scenario files, sub-component files, split CSS files
**Files to modify:** 7 monolithic files

---

### D4. Dead Code to Remove

**Context:** ~1,120 lines of dead code across 7 files. 4 hooks have zero imports. 3 files are git-deleted but physically on disk. Removing these reduces maintenance burden and bundle size.

---

#### D4.1 `[x]` Delete dead hooks — ~200L

**Zero-import hooks** (verified by grep across entire `src/`):

| File | Lines | Why dead |
|---|---|---|
| `src/hooks/useAgents.ts` | ~50 | Replaced by direct API calls in AgentFleet |
| `src/hooks/useDashboard.ts` | ~50 | Replaced by per-page data fetching |
| `src/hooks/useKnowledge.ts` | ~50 | Replaced by direct API calls in KnowledgeEntries/Graph |
| `src/hooks/useTerminalSession.ts` | ~50 | Replaced by useTerminal |

**Acceptance:** Files physically deleted. `npm run build` passes. `tsc --noEmit` passes.
**Files to delete:** 4 hook files listed above

---

#### D4.2 `[x]` Delete git-deleted files from disk — ~920L

| File | Lines | What it was |
|---|---|---|
| `src/hooks/useApiWithFallback.ts` | 148 | Server-probe + demo-data fallback wrapper |
| `src/lib/bench-demo-data.ts` | 252 | Offline benchmark seed data |
| `src/lib/demo-data.ts` | 522 | Offline seed data for all pages |
| `src/lib/model-catalog.ts` | ~50 | Static model list (replaced by API) |

**Acceptance:** Files removed from disk. `ls` confirms absence. `npm run build` still passes (these should already be unreferenced after git deletion, except AppShell → useApiWithFallback).
**Prerequisite:** A3.3 must be done first (fix AppShell import).
**Files to delete:** 4 files listed above

---

#### D4.3 `[x]` Fix AppShell import of deleted hook

**Current:** `AppShell.tsx` imports `useApiWithFallback` which is git-deleted but physically present.
**Acceptance:** Import changed to `useLiveApi`. Build passes without `useApiWithFallback.ts` on disk.
**Files to modify:** `src/components/AppShell.tsx`
**Prerequisite for:** D4.2

---

## E. Accessibility (from 04-DESIGN-SYSTEM, 10-UX-PHILOSOPHY)

**Context:** The app has major a11y gaps: zero `prefers-reduced-motion`, no focus rings, no keyboard navigation, minimal ARIA. These affect users with motor impairments, vestibular disorders, and screen reader users.

**Spec reference:** `10-UX-PHILOSOPHY.md` §2 AP-12, `04-DESIGN-SYSTEM.md` §4

---

#### E1 `[x]` Add `prefers-reduced-motion` — HIGH

**Duplicate of B4.4 — see that item for full detail.**

---

#### E2 `[x]` Add ARIA labels to canvas elements

**Current:** Most canvases (`KnowledgeGraph`, `AgentFleet` topology, `CostRace`, `GateWaterfall`, `DreamPhaseViz`, all chart components) have no `role` or `aria-label`.
**`useCanvasSetup`** is used by 32 components — it could inject ARIA attributes.
**Acceptance:** Every `<canvas>` has `role="img"` and descriptive `aria-label`. `grep -rn '<canvas' src/` shows all have labels.
**Files to modify:** `src/hooks/useCanvasSetup.ts` (add ARIA support), or each canvas consumer individually

---

#### E3 `[x]` Add keyboard nav to interactive tables

**Current:** Tables in `CascadeRouter`, `KnowledgeEntries`, `Bench` history are mouse-only.
**Acceptance:** `tabIndex={0}` on rows. Arrow keys navigate. Enter activates. Focus ring visible.
**Files to modify:** `CascadeRouter.tsx`, `KnowledgeEntries.tsx`, `Bench.tsx`

---

#### E4 `[x]` Add `--focus-ring` token and apply

**Duplicate of B1.8 — see that item for detail.**
**Additional acceptance:** Applied globally via `rosedust.css`:
```css
:focus-visible {
  outline: 2px solid var(--rose-bright);
  outline-offset: 2px;
  box-shadow: 0 0 0 4px rgba(220,165,189,0.2);
}
```

---

#### E5 `[x]` Add keyboard shortcuts

**Current:** Zero keyboard shortcuts.
**Target shortcuts:**
| Key | Action | Context |
|---|---|---|
| `Space` | Play/pause pipeline | Orchestrate |
| `N` | Next step | Orchestrate (step mode) |
| `R` | Reset scenario | Orchestrate |
| `1-3` | Select scenario | Orchestrate |
| `?` | Show help overlay | Global |
| `T` | Toggle terminal | Orchestrate/Build |
| `⌘K` | Command palette | Global |

**Acceptance:** Shortcuts work on every page. `?` shows cheatsheet overlay. No conflicts with browser defaults.
**Files to create:** `src/hooks/useKeyboardShortcuts.ts`, `src/components/HelpOverlay.tsx`

---

## F. Performance (from 02-ARCHITECTURE, 04-DESIGN-SYSTEM)

**Spec reference:** `02-ARCHITECTURE.md` §7, `10-UX-PHILOSOPHY.md` §1.12 principle 1

---

#### F1 `[x]` Code-split Three.js

**Current:** Three.js (~500kB) imported eagerly by `HeroScene.tsx` (512L). Loaded on every page even if user never sees the hero scene.
**Acceptance:** `React.lazy(() => import('./HeroScene'))` with `<Suspense>` fallback. Three.js chunk loads only when hero scene mounts. Verify with `npm run build && ls -la dist/assets/` — separate chunk for Three.js.
**Files to modify:** `src/App.tsx` or wherever HeroScene is imported

---

#### F2 `[x]` Code-split xterm.js

**Current:** xterm.js (~200kB) imported eagerly by `useTerminal.ts`. Loaded on every page.
**Acceptance:** Terminal component lazy-loaded. xterm.js in separate chunk.
**Files to modify:** Import site of Terminal component

---

#### F3 `[x]` Bundle size audit

**Current:** Unknown total bundle size.
**Acceptance:** `npm run build` output shows main chunk < 200kB gzipped. Run `npx vite-bundle-visualizer` and document top 10 chunks.
**No files to modify** — this is a measurement task. Creates action items if budget exceeded.

---

#### F4 `[x]` Animation frame budget

**Current:** Unknown frame timing. Some pages run 8+ concurrent CSS animations (e.g., Agent Fleet with multiple pulse + breathing animations).
**Acceptance:** DevTools Performance tab shows ≤ 4ms/frame. Max 8 concurrent CSS animations per page.
**Files to modify:** Pages with excessive animations (AgentFleet, KnowledgeGraph, Explorer)

---

#### F5 `[x]` Selector optimization

**Current:** No `shallow` equality on multi-field Zustand selectors.
**Prerequisite:** DataHub must exist first (A1.1).
**Acceptance:** Multi-field selectors use `useDataHub(selector, shallow)` to prevent unnecessary re-renders. Verify with React DevTools Profiler — no wasted renders on unrelated state changes.
**Files to modify:** All DataHub consumers (after A1.6)

---

## G. Cell System (from 02-ARCHITECTURE §4)

**Context:** The Cell system is the core visual primitive — "everything is a Cell." Cells are universal containers with status ring, identity, actions, and connections. Current components use ad-hoc card layouts. Cell unifies: task cards, agent cards, bench run cards, knowledge entries, episodes, gate results, signals.

**Spec reference:** `02-ARCHITECTURE.md` §4 (Cell architecture), `09-DESIGN-PRIMITIVES.md` §2

**Prerequisite:** B1.9 (cell tokens) should be done first.

---

#### G1 `[x]` Create `Cell.tsx` base container

**Target:** Status ring (color from entity state) + identity area (icon/avatar + title) + metric area + action bar + connection ports. Glass background with specular highlight.
**Acceptance:** `<Cell status="active" title="Task 1" />` renders glass card with green status ring. Hover lifts 3px with shadow.
**Files to create:** `src/components/Cell.tsx`, `src/components/Cell.css`
**Spec reference:** `02-ARCHITECTURE.md` §4.1, `09-DESIGN-PRIMITIVES.md` §2.1

---

#### G2 `[x]` Create `CellGrid.tsx`

**Target:** Responsive grid with staggered entrance animation. Auto-columns based on container width.
**Acceptance:** `<CellGrid>{cells}</CellGrid>` renders responsive grid. Cells stagger-enter with 50ms delay. Reflows smoothly on resize.
**Files to create:** `src/components/CellGrid.tsx`

---

#### G3 `[x]` Create `CellTimeline.tsx`

**Target:** Chronological list with time markers (relative timestamps). Used for episodes, events, logs.
**Acceptance:** `<CellTimeline items={episodes} />` renders vertical timeline. Time markers group by day/hour.
**Files to create:** `src/components/CellTimeline.tsx`

---

#### G4 `[x]` Create `CellBoard.tsx`

**Target:** Kanban columns: pending → active → done → failed. Used for task boards.
**Acceptance:** `<CellBoard tasks={tasks} />` renders 4 columns. Tasks animate between columns on state change.
**Files to create:** `src/components/CellBoard.tsx`

---

#### G5 `[x]` Create `CellDetail.tsx`

**Target:** Expanded detail panel with shared element transition from CellGrid/CellBoard item. Used for task detail, bench run detail.
**Acceptance:** Clicking a Cell in CellGrid opens CellDetail with smooth transition. Back button returns with reverse transition.
**Prerequisite:** B4.6 (Motion library) for shared element transitions.
**Files to create:** `src/components/CellDetail.tsx`

---

#### G6 `[x]` Create `CellGraph.tsx`

**Target:** Force-directed graph of connected cells. Used for topology, knowledge graph, dependency DAGs.
**Acceptance:** `<CellGraph nodes={agents} edges={connections} />` renders interactive graph. Nodes are Cell components. Edges animate on data flow.
**Files to create:** `src/components/CellGraph.tsx`

---

#### G7 `[x]` Create entity renderers

**Target:** Typed render functions that map domain entities to Cell props. One per entity type:

| Entity | Status source | Identity | Metrics |
|---|---|---|---|
| Task | task.status | task.title + agent badge | duration, model, cost |
| Agent | agent.health | SpectreAvatar + name | turns, tokens, cost |
| Plan | plan.status | plan.slug | task count, progress % |
| Episode | — | agent + timestamp | turn count, tokens |
| BenchRun | run.status | run.name | score, cost, time |
| Knowledge | entry.tier | domain + topic | citations, confidence |
| Gate | gate.verdict | gate.name | threshold, actual |
| Signal | — | signal.kind | hash, timestamp |

**Acceptance:** `<Cell entity={task} />` renders correctly for each entity type. Type-safe — passing wrong entity type causes TS error.
**Files to create:** `src/components/entity-renderers.ts`

---

## Summary

| Category | Items | Not Started | Partial | Done |
|----------|-------|-------------|---------|------|
| A. Architecture | 16 | 8 | 1 | 7 |
| B. Design System | 17 | 2 | 0 | 15 |
| C. Agent Model | 8 | 8 | 0 | 0 |
| D. UX & State | 27 | 13 | 5 | 9 |
| E. Accessibility | 5 | 5 | 0 | 0 |
| F. Performance | 5 | 5 | 0 | 0 |
| G. Cell System | 7 | 7 | 0 | 0 |
| **Total** | **85** | **48** | **6** | **31** |

**Updated 2026-04-29.** Tier 0 (tokens, typography, dead code, rgba cleanup, keyframe consolidation) and Tier 1 transport layer (api.ts, sse.ts, ws.ts, types.ts) complete. Pipeline state machine types created. Dashboard inline style extraction done for 5/10 pages.

---

## Priority Order

### Tier 0 — Foundation (must do first, unblocks everything)
1. **B1.6, B1.7, B1.8** — Motion easing + duration + focus ring tokens
2. **B4.1** — Eliminate `transition: all` (30 violations)
3. **B4.4 / E1** — Add `prefers-reduced-motion` (complete a11y gap)
4. **A3.3, A3.4, D4.1, D4.2, D4.3** — Delete dead code + stale files (~1,120 lines)

### Tier 1 — Architecture (enables all feature work)
5. **A1.1-A1.7** — DataHub + transport + thin selectors
6. **A2.1-A2.5** — Pipeline state machine + Activity Strip

### Tier 2 — Visual System
7. **B1.3** — Status glow tokens
8. **B2.1-B2.3** — Replace 283 raw rgba + 6 hex values
9. **B4.2** — Consolidate duplicate keyframes
10. **G1-G7** — Cell system

### Tier 3 — Agent Identity
11. **C1-C8** — Spectre system + agent attribution + topology

### Tier 4 — UX Polish
12. **D1.1-D1.13** — Loading/empty/error state coverage (13 pages)
13. **D2.1-D2.10** — Inline style reduction (create 8 CSS files)
14. **D3.1-D3.7** — Monolithic file splits

### Tier 5 — Performance & Advanced A11y
15. **F1-F2** — Bundle splitting (Three.js + xterm.js)
16. **F3-F5** — Bundle audit + frame budget + selector optimization
17. **E2-E5** — Canvas ARIA + keyboard nav + shortcuts

---

## Cross-References

| Spec Doc | Checklist Sections |
|---|---|
| `02-ARCHITECTURE.md` | A1, A2, F, G |
| `03-REALTIME-DATA.md` | A1.3, A1.4, A1.5 |
| `04-DESIGN-SYSTEM.md` | B1-B4, E |
| `05-PAGES.md` | D1, D3 |
| `06-AGENT-MODEL.md` | C1-C8 |
| `09-DESIGN-PRIMITIVES.md` | G1-G7 |
| `10-UX-PHILOSOPHY.md` | A3, D1-D4, E5 |

| Duplicate Items (cross-listed for discoverability) |
|---|
| A1.7 = A3.2 (server health singletons) |
| A3.3 = D4.3 (AppShell useApiWithFallback import) |
| B1.8 = E4 (focus ring token) |
| B4.4 = E1 (prefers-reduced-motion) |
| D3.6 depends on A1.1 (useBench becomes selector after DataHub) |
