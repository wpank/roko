# 08. Codebase Audit Findings

Comprehensive audit of every file in `demo/demo-app/src/`. Organized by category with specific file paths, line numbers, and recommended actions.

---

## 1. Dead Code — Delete Immediately (~813 lines)

### 1.1 Dead Hooks (6 files, ~348 lines)

| File | Lines | Why Dead |
|------|-------|----------|
| `hooks/useSSE.ts` | ~80 | Zero imports. Replaced by `useEventStream.ts` and `useBenchSSE` |
| `hooks/useAgents.ts` | ~45 | Zero imports. Was for `/api/managed-agents`, never wired |
| `hooks/useDashboard.ts` | ~60 | Zero imports. Was for dashboard polling, dashboard page removed |
| `hooks/useKnowledge.ts` | ~55 | Zero imports. Explorer.tsx fetches knowledge inline |
| `hooks/useSweBench.ts` | ~70 | Zero imports. SWE-bench feature never shipped |
| `hooks/useDemoMode.ts` | ~38 | Zero imports. Demo mode handled inline in Demo.tsx |

**Action**: Delete all 6 files. Run `npx tsc --noEmit` to verify nothing breaks.

### 1.2 Dead Components (6 files, ~465 lines)

| File | Lines | Why Dead |
|------|-------|----------|
| `components/WorkflowConstellation.tsx` | ~120 | Zero imports. Was Three.js workflow viz, never connected |
| `components/StatCard.tsx` | ~45 | Zero imports. Replaced by `Mosaic.tsx` cells |
| `components/LiveIndicator.tsx` | ~35 | Zero imports. Status dots are inline everywhere |
| `components/Skeleton.tsx` | ~40 | Zero imports. Loading states are inline or missing |
| `components/CrushedBar.tsx` | ~85 | Zero imports. Replaced by inline SVG bars in Bench.tsx |
| `components/ModelPicker.tsx` | ~140 | Zero imports. Model selection is inline in Builder.tsx |

**Action**: Delete all 6 files. Run `npx tsc --noEmit` to verify.

### 1.3 Dead Exports in Live Files

| File | Dead Export | Reason |
|------|-----------|--------|
| `hooks/useChain.ts` | `useChain` function, `mirageRpc` function | Zero external callers |
| `lib/scenarios.ts` | `stripAnsi` function | Zero callers |
| `lib/scenarios.ts` | `SCENARIO_MAP` map | Zero callers (scenarios accessed by array index) |
| `lib/scenarios.ts` | `resetScenarioState()` function | Zero callers |
| `lib/workflow-api.ts` | Old `createWorkflowSubscription` (if still exported) | Replaced by `useWorkspace` |

**Action**: Remove dead exports. Keep live files.

---

## 2. Memory Leaks — Fix Immediately

### 2.1 Module-Level Health Poll (Critical)

**File**: `hooks/useLiveApi.ts:43`
```typescript
// Module-level interval — never cleared
let _healthPoll: ReturnType<typeof setInterval> | undefined;
```
This interval is created at module scope. When the component using `useLiveApi` unmounts, the interval keeps running. If the component remounts, a second interval starts. After 10 mount/unmount cycles, 10 concurrent intervals poll `/api/health`.

**Fix**: Move to `useEffect` with cleanup:
```typescript
useEffect(() => {
  const id = setInterval(() => pollHealth(), 30000);
  return () => clearInterval(id);
}, []);
```

### 2.2 Unbounded Event Array (Medium)

**File**: `hooks/useBenchSSE.ts:55`
```typescript
// Events array grows without bound during bench runs
setEvents(prev => [...prev, parsed]);
```
A bench run with 100 tasks pushes 100+ events into state. Multiple runs accumulate. No GC.

**Fix**: Ring buffer with max 500 entries:
```typescript
setEvents(prev => [...prev.slice(-499), parsed]);
```

### 2.3 Module-Level CSS Variable Cache (Low)

**File**: `components/KnowledgeFlowPanel.tsx` (top of file)
```typescript
// Module-level mutable cache — never invalidated
const cssVarCache: Record<string, string> = {};
```
If design tokens change at runtime (e.g., theme switch), cached values are stale.

**Fix**: Use `getComputedStyle()` directly (it's fast) or invalidate on theme change event.

---

## 3. Bugs — Fix Before Refactor

### 3.1 Shell Command Injection (Security)

**File**: `pages/Builder.tsx` (in handleSend)
```typescript
handle.execCmd(`roko run "${prompt}" --model ${selectedModel}`);
```
If `prompt` contains double quotes or backticks, the shell command breaks or executes arbitrary code.

**Fix**: Escape the prompt or pass via stdin:
```typescript
const escaped = prompt.replace(/["\\`$]/g, '\\$&');
handle.execCmd(`roko run "${escaped}" --model ${selectedModel}`);
```

### 3.2 Stale Callback Dependency

**File**: `pages/Builder.tsx` (in useCallback for handleSend)
The `handleSend` callback captures `selectedModel` but the `useCallback` deps may be stale if model changes between renders.

**Fix**: Include `selectedModel` in useCallback dependency array, or use a ref.

### 3.3 Busy-Polling in Terminal Hooks

**File**: `hooks/useTerminal.ts` (in `waitForPrompt` and `waitForMarker`)
```typescript
// Polls every 30ms in a tight loop
while (!found) {
  await sleep(30);
  // check condition...
}
```
This burns CPU unnecessarily. Should use an event-driven approach.

**Fix**: Use xterm's `onData` or `onLineFeed` events with a Promise resolver instead of polling.

### 3.4 Duplicate Health Polling

**Files**: `hooks/useServerHealth.ts` AND `hooks/useLiveApi.ts`
Both independently poll `GET /api/health` on their own intervals. Any component using both (e.g., via AppShell) sends double traffic.

**Fix**: Consolidate into DataHub's `serverStatus` field (Phase 1 fix).

---

## 4. Duplicated Utilities — Extract to Shared Modules

### 4.1 `hexToRgba` — Duplicated 6+ Times

Found in: `CostRace.tsx`, `HeroParticleField.tsx`, `KnowledgeFlowPanel.tsx`, `DreamPhaseViz.tsx`, `MatrixRaceTrack.tsx`, `HeroScene.tsx`

Each copy is functionally identical:
```typescript
function hexToRgba(hex: string, alpha: number): string {
  const r = parseInt(hex.slice(1, 3), 16);
  // ...
}
```

**Action**: Create `lib/color.ts`:
```typescript
export function hexToRgba(hex: string, alpha: number): string { ... }
export function getCssVar(name: string): string { ... }
```
Replace all 6+ inline copies with import.

### 4.2 `shortModel` — Duplicated 4 Times

Found in: `BenchRunDetail.tsx`, `MatrixDetailView.tsx`, `CostRace.tsx`, `Bench.tsx`

Each copy strips provider prefix from model names:
```typescript
function shortModel(m: string): string {
  return m.replace(/^(anthropic|openai|google)\//, '');
}
```

**Action**: Create in `lib/format.ts`:
```typescript
export function shortModel(model: string): string { ... }
```

### 4.3 `fmtUptime` — Duplicated 3 Times

Found in: `Explorer.tsx` (3 separate copies)

```typescript
function fmtUptime(secs: number): string {
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  return `${h}h ${m}m`;
}
```

**Action**: Add to `lib/format.ts`.

### 4.4 `relativeTime` — Duplicated 2 Times

Found in: `Explorer.tsx`, `BenchRunDetail.tsx`

**Action**: Add to `lib/format.ts`.

### 4.5 Canvas DPR/Resize Boilerplate — Duplicated 15+ Times

Found in every file that uses `<canvas>`: `HeroParticleField.tsx`, `CostRace.tsx`, `KnowledgeFlowPanel.tsx`, `DreamPhaseViz.tsx`, `MatrixRaceTrack.tsx`, `HeroScene.tsx`, `Explorer.tsx` (5 canvases), `TokenVelocitySparkline.tsx`, `BenchRunDetail.tsx` (2 canvases), `Bench.tsx` (2 canvases)

Each repeats:
```typescript
const dpr = window.devicePixelRatio || 1;
canvas.width = rect.width * dpr;
canvas.height = rect.height * dpr;
ctx.scale(dpr, dpr);
```

**Action**: Create `hooks/useCanvasSetup.ts`:
```typescript
export function useCanvasSetup(
  canvasRef: RefObject<HTMLCanvasElement>,
  draw: (ctx: CanvasRenderingContext2D, w: number, h: number) => void
) { ... }
```
Handles DPR, resize observer, cleanup. Replaces 15+ copies of boilerplate.

### 4.6 Color Palette Maps — Duplicated 3+ Times

Found in: `CostRace.tsx`, `Bench.tsx`, `MatrixDetailView.tsx`

```typescript
const DOMAIN_COLORS = { /* same colors */ };
const ROLE_COLORS = { /* same colors */ };
```

**Action**: Create `lib/palette.ts`:
```typescript
export const DOMAIN_COLORS: Record<string, string> = { ... };
export const ROLE_COLORS: Record<string, string> = { ... };
export const MODEL_COLORS: Record<string, string> = { ... };
```

---

## 5. Monolithic Files — Split Into Focused Modules

### 5.1 `scenarios.ts` — 1,910 Lines (Largest File)

**Problem**: 12 scenario definitions + terminal orchestration + workspace management + all utility functions in one file. `ScenarioContext` has 16 fields.

**Split plan**:
```
lib/scenarios.ts (1910L) →
  lib/scenario-registry.ts   (~200L) — ScenarioConfig[], ScenarioContext type
  lib/scenario-runners/
    prd-pipeline.ts           (~150L) — prdPipelineScenario
    model-race.ts             (~150L) — modelRaceScenario
    ... (one per scenario)
  lib/terminal-orchestration.ts (~100L) — enterWorkspace, resolveRoko, shared utils
```

### 5.2 `useBench.ts` — 533 Lines, 17 useState Calls

**Problem**: Manages runs, filtering, sorting, SSE connection, matrix config, comparison — all in one hook.

**Split plan**:
```
hooks/useBench.ts (533L) →
  hooks/useBenchRuns.ts     (~150L) — run list + CRUD
  hooks/useBenchFilter.ts   (~100L) — sort/filter state
  hooks/useBenchMatrix.ts   (~150L) — matrix builder + progress
```

### 5.3 `Demo.tsx` — 832 Lines

**Problem**: Scenario selection, phase tracking, terminal management, sidebar rendering, reset logic all inline. 3 copies of default agent info reset.

**Split plan**:
```
pages/Demo.tsx (832L) →
  pages/Demo.tsx               (~200L) — orchestration shell
  components/ScenarioSelector.tsx (~100L) — card grid + description
  components/PhaseRailBar.tsx    (~80L) — phase indicator strip
  components/DemoSidebar.tsx     (~150L) — context-sensitive sidebar
  hooks/useDemoState.ts          (~100L) — phase + scenario state machine
```

### 5.4 `Bench.tsx` — 701 Lines, 7 Inline Tabs

**Problem**: All 7 tab contents rendered inline. No lazy loading. Inline IIFEs for data computation.

**Split plan**: Extract each tab body to a component:
```
pages/Bench.tsx (701L) →
  pages/Bench.tsx              (~150L) — tab shell + routing
  components/bench/Overview.tsx   — quick stats + recent runs
  components/bench/RunList.tsx    — sortable run table
  components/bench/Pareto.tsx     — scatter chart
  components/bench/Compare.tsx    — side-by-side
  components/bench/Matrix.tsx     — matrix builder
  components/bench/Insights.tsx   — learning insights
```

### 5.5 `Explorer.tsx` — 859 Lines, 5 Canvas Refs

**Problem**: 222-line `drawTimeline()` inline, `fmtUptime()` duplicated 3x, 5 sparkline canvases.

**Split plan**:
```
pages/Explorer.tsx (859L) →
  pages/Explorer.tsx           (~250L) — tab layout
  components/explorer/TimelineCanvas.tsx  (~150L)
  components/explorer/SparklineCanvas.tsx (~80L)
  components/explorer/HealthMosaic.tsx    (~100L)
  components/explorer/ProviderTable.tsx   (~100L)
```

### 5.6 `BenchRunDetail.tsx` — 654 Lines, 3 Inline Components

**Problem**: `CostBreakdownChart` (228L), `TokenFlowChart` (126L), `OutputPreviewPanel` (80L) are all declared inline.

**Split plan**: Extract each to its own file under `components/bench/`.

---

## 6. CSS Inconsistencies

### 6.1 Duplicate @keyframes (4 Conflicts)

| Keyframe | Copies | Files | Issue |
|----------|--------|-------|-------|
| `fadeIn` | 2 | `rosedust.css`, `PrdPipelinePanel.css` | Different durations (200ms vs 300ms) |
| `pulse-dot` | 3 | `TopNav.css`, `Explorer.css`, `PrdPipelinePanel.css` | ALL THREE DIFFER in timing/opacity |
| `benchlive-pulse` | 1 def, 3 refs | Defined in `Bench.css`, used in `MatrixBuilder.css`, `MatrixDetailView.css` | Cross-file dependency — breaks if Bench.css not loaded |
| `conn-blink` / `term-dot-blink` | 2 | `TopNav.css`, `TerminalPane.css` | Identical behavior, different names |

**Action**: Consolidate all keyframes into `styles/animations.css`:
```css
/* Single source of truth */
@keyframes fade-in { from { opacity: 0 } to { opacity: 1 } }
@keyframes pulse-dot { 0%, 100% { opacity: 1 } 50% { opacity: 0.35 } }
@keyframes status-blink { 0%, 100% { opacity: 1 } 50% { opacity: 0 } }
```

### 6.2 Hardcoded Colors (~130 in TSX Files)

Found across 25+ TSX files. Examples:
- `#2dd4bf` (teal) — used 8+ times inline instead of `var(--status-active)`
- `#4ade80` (green) — used 6+ times instead of `var(--status-success)`
- `#fb7185` (rose) — used 5+ times instead of `var(--status-error)`
- `#fbbf24` (amber) — used 4+ times instead of `var(--status-warning)`
- `rgba(255,255,255,0.07)` — border color, used 20+ times instead of `var(--border)`
- `#0a0810`, `#080810`, `#060608` — background variants, ~15 uses instead of tokens

**Action**: Replace ALL hardcoded hex/rgba with CSS custom property references. For canvas contexts where CSS vars aren't directly available, use `getCssVar()` from `lib/color.ts`.

### 6.3 Inline Styles (~330+ Instances)

Heaviest offenders:
| File | Inline `style={{}}` Count |
|------|--------------------------|
| `Explorer.tsx` | ~45 |
| `Demo.tsx` | ~35 |
| `Bench.tsx` | ~30 |
| `BenchRunDetail.tsx` | ~28 |
| `PrdPipelinePanel.tsx` | ~25 |
| `CostRace.tsx` | ~20 |
| `MatrixDetailView.tsx` | ~18 |

Many are for simple layout (`display: 'flex'`, `gap`, `padding`) that should be CSS classes. Others are dynamic values (widths from data) which are fine.

**Action**:
- Static layout styles → CSS classes (in component's `.css` file)
- Dynamic values → keep as inline styles
- Theme colors → CSS custom properties
- Target: reduce inline styles by ~60% (from ~330 to ~130)

### 6.4 `Settings.css` — Alien Variable Namespace

Uses completely non-ROSEDUST variables:
```css
--surface-0, --surface-1, --surface-2, --accent, --accent-dim, --border-dim
```
These don't exist in `rosedust.css`. The Settings page silently falls back to browser defaults.

**Action**: Rewrite `Settings.css` to use ROSEDUST tokens (`--bg-void`, `--bg-raised`, `--rose`, `--border`, etc.).

### 6.5 `TerminalPane.css` — Hardcoded Background

```css
.terminal-pane { background: #0e0c10; }
```
Should be `var(--bg-deeper)` or `var(--bg-void)`.

### 6.6 Scrollbar Conflict

Both `global.css` and `rosedust.css` define `::-webkit-scrollbar` styles with different values. Browser uses last-loaded, which varies with import order.

**Action**: Remove scrollbar styles from `global.css`, keep only in `rosedust.css`.

### 6.7 Danger Color Mismatch

`PrdPipelinePanel.css` uses `#cc6f6f` as danger fallback, but `rosedust.css` defines `--danger: #cc5555`. The fallback doesn't match the token.

**Action**: Replace hardcoded fallback with `var(--danger)` (no fallback needed if rosedust.css is always loaded).

---

## 7. Accessibility Gaps

### 7.1 Canvas Charts — No ARIA

Every `<canvas>` element (15+ instances) lacks `role="img"` and `aria-label`. Screen readers see nothing.

**Action**: Add to every canvas:
```tsx
<canvas ref={ref} role="img" aria-label="Cost breakdown chart showing..." />
```

### 7.2 Interactive Tables — No Keyboard Nav

`TaskTable`, `ChainActivityPanel`, `BenchRunList` — rows are clickable via `onClick` but not focusable via keyboard.

**Action**: Add `tabIndex={0}`, `onKeyDown` (Enter/Space = click), `role="row"`.

### 7.3 Color-Only Status Indicators

Several status dots use color alone (green/red/amber) without text or icon.

**Action**: Pair every status color with an icon (`✓`, `✕`, `◉`, `○`) per the design system spec.

---

## 8. Architecture Smells

### 8.1 Three SSE Implementations

| File | Endpoint | Pattern |
|------|----------|---------|
| `hooks/useSSE.ts` | `/api/events` | Generic EventSource wrapper (DEAD) |
| `hooks/useBenchSSE.ts` | `/api/bench/events` | Bench-specific SSE |
| `hooks/useEventStream.ts` | `/api/events` | Context-based event stream |

All three do the same thing: connect to SSE, parse JSON, dispatch. Unified transport (Phase 1) replaces all.

### 8.2 `useTerminalSession.ts` Is Not a React Hook

Despite living in `hooks/`, `useTerminalSession.ts` exports pure functions (`setupWorkspace`, `joinWorkspace`, `enterWorkspace`, `resolveRoko`, `waitForPrompt`) — none use React hooks. It's a utility module.

**Action**: Move to `lib/terminal-session.ts`.

### 8.3 `useWorkspace.ts` Uses Raw `fetch`

Uses `fetch()` directly instead of the `useLiveApi` abstraction used everywhere else. Inconsistent error handling and no retry logic.

**Action**: Refactor to use `RokoApi` client (Phase 1) or at minimum align with `useLiveApi` pattern.

### 8.4 Five Different Error Handling Patterns

1. `try/catch` with `console.error` (most hooks)
2. `try/catch` with `setError(e.message)` (some hooks)
3. `.catch(() => {})` swallow (useLiveApi)
4. No error handling at all (several canvas draw functions)
5. `result || fallback` pattern (workflow-api.ts)

**Action**: Standardize on DataHub pattern: fetch returns `T | null`, errors logged to DataHub error slice, components use `<DataSurface>` wrapper.

### 8.5 No Shared Data Layer

Every page fetches its own data independently:
- `Explorer.tsx` calls `/api/health`, `/api/episodes`, `/api/learn/cascade-router`, `/api/providers/health`, `/api/metrics/c_factor` in its own `useEffect`
- `Bench.tsx` calls `/api/bench/runs`, `/api/bench/cost-summary` in its own `useEffect`
- `Demo.tsx` calls `/api/health` in its own `useEffect`

Result: Same endpoint called 2-4 times across different pages. No shared cache.

**Action**: All data flows through DataHub (Phase 1). Pages subscribe to slices.

---

## 9. Missing Error Boundaries

### 9.1 Three.js Components Can Crash

`HeroScene.tsx`, `WorkflowConstellation.tsx` (dead), `HeroParticleField.tsx` — WebGL context failures crash the entire React tree.

**Action**: Wrap all Three.js/canvas-heavy components in `<ErrorBoundary>`.

### 9.2 SSE Parse Failures Unhandled

In `useBenchSSE.ts` and `useEventStream.ts`, `JSON.parse(event.data)` can throw on malformed data. The catch is empty or missing.

**Action**: Catch parse errors, log to console, skip the event. Never crash on bad server data.

---

## 10. Performance Concerns

### 10.1 Bundle Size

Three.js (`@react-three/fiber`, `@react-three/drei`) adds ~500kB to the bundle but is only used by `HeroScene.tsx` (a background particle field). This should be lazy-loaded.

xterm.js adds ~200kB but is only used when terminals are visible.

**Action**: Code-split both via `React.lazy()`:
```typescript
const HeroScene = React.lazy(() => import('./components/HeroScene'));
const Terminal = React.lazy(() => import('./components/TerminalPane'));
```

### 10.2 Unnecessary Re-renders

`Demo.tsx` has ~15 `useState` calls. Any state change re-renders the entire 832-line component including all children. No `useMemo` or `React.memo` optimization.

Similarly `Bench.tsx` (17 useState) and `Explorer.tsx` (~12 useState).

**Action**: Phase 1 DataHub + Phase 3 scene rebuilds fix this structurally.

### 10.3 Canvas Redraw Without RAF Batching

Several canvas components redraw on every state change without `requestAnimationFrame` batching. If multiple state updates happen in one frame, canvas redraws multiple times.

**Action**: Wrap canvas draw calls in `requestAnimationFrame`, skip if a frame is already pending.
