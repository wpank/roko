# Prioritized Solutions

Fixes ordered by impact on VC demo quality. Each item references specific findings from the other docs.

---

## Tier 1: Demo-Breaking (Must fix)

These prevent a successful live demo.

### S1. Fix undefined CSS variables in dashboard pages

**Refs:** V1, 04-VISUAL-AUDIT
**Files:** `pages/dashboard/Layout.tsx`, `CascadeRouter.tsx`, `KnowledgeEntries.tsx`, `ShareView.tsx`
**Effort:** 30 minutes

Replace all undefined CSS variable references:
```
--glass-2-border  →  --glass-border
--raised          →  --bg-raised
--font-sans       →  --sans
--font-serif      →  --display  (or --serif)
--text            →  --text-primary
--fail            →  --rose-bright
```

This immediately fixes invisible text and broken styling on 4 dashboard pages.

---

### S2. Wire speed control to actually affect demo timing

**Refs:** P1, P2, A1, 03-PRESENTER-CONTROL
**Files:** `hooks/useTerminalSession.ts`, `lib/scenarios.ts`, `pages/Demo.tsx`
**Effort:** 2 hours

1. Create `demoSleep(ms, speed)` that divides delay by speed multiplier
2. Replace all `rawSleep()` calls in scenarios with `demoSleep()`
3. Pass `globalSpeed` through to `typeVisibleCommandAndWait` character delay
4. Verify speed button actually changes pacing

---

### S3. Add `waitForStep()` to race/providers/explore scenarios

**Refs:** P4, 03-PRESENTER-CONTROL
**Files:** `lib/scenarios.ts`
**Effort:** 1 hour

Add `await playback.waitForStep()` at minimum:
- Before `Promise.all` in `race` scenario
- Before `Promise.all` in `providers` scenario
- Before parallel setup in `explore` scenario
- After completion in each (for results discussion)

---

### S4. Remove or placeholder the Jobs nav link

**Refs:** 04-VISUAL-AUDIT, 05-WORKFLOW-E2E
**Files:** `components/TopNav.tsx`, optionally `main.tsx`
**Effort:** 10 minutes

Either remove "Jobs" from `TopNav` or add a route with a "Coming Soon" placeholder component.

---

### S5. Switch KnowledgeEntries to `useApiWithFallback`

**Refs:** V2, 04-VISUAL-AUDIT
**Files:** `pages/dashboard/KnowledgeEntries.tsx`
**Effort:** 15 minutes

Change `useApi` to `useApiWithFallback` and add demo data fallback. Matches all sibling dashboard pages.

---

## Tier 2: High Impact Polish

These significantly improve the demo experience.

### S6. Fix WebSocket listener accumulation

**Refs:** B1, A4, 02-RUNTIME-BUGS
**Files:** `hooks/useTerminal.ts`
**Effort:** 45 minutes

Move `term.onData()` and `term.onResize()` registration outside `connectWs()`. Store `IDisposable` for cleanup. Use `wsRef.current` inside callbacks.

---

### S7. Fix `useServerHealth` first-check false positive

**Refs:** B10, P7, 02-RUNTIME-BUGS
**Files:** `hooks/useServerHealth.ts`
**Effort:** 20 minutes

On first check failure, set status to `'checking'` or `'disconnected'` instead of `'connected'`. Update `Demo.tsx` guard to handle `'checking'` state gracefully.

---

### S8. Fix ChainView setTimeout/setInterval leak

**Refs:** B7, 02-RUNTIME-BUGS
**Files:** `pages/dashboard/ChainView.tsx`
**Effort:** 15 minutes

Store `setTimeout` return in `timeoutRef`. Clear both `intervalRef` and `timeoutRef` in effect cleanup.

---

### S9. Add loading skeletons to dashboard pages

**Refs:** V4, 04-VISUAL-AUDIT
**Files:** All dashboard page components
**Effort:** 2 hours

Add shimmer/skeleton placeholders while API data is loading. Prevents the "blank page then data pops in" flash.

---

### S10. Add empty states to all data views

**Refs:** V3, 04-VISUAL-AUDIT
**Files:** Multiple dashboard/explorer components
**Effort:** 2 hours

Replace empty voids with informative messages:
- Provider Health: "No providers configured"
- Agent Grid: "No agents registered"
- Knowledge Graph: "No knowledge shards yet"
- Explorer tabs: Per-tab messages
- Bench History: Already has one (good)

---

### S11. Add navigation links to BenchRunDetail and BenchCompare

**Refs:** 05-WORKFLOW-E2E
**Files:** `pages/Bench.tsx`
**Effort:** 30 minutes

Add "View" link in history table rows → `/bench/run/:id`. Add "Compare" button in history tab → `/bench/compare`.

---

### S12. Guard stale callbacks on scenario switch

**Refs:** P6, 03-PRESENTER-CONTROL
**Files:** `pages/Demo.tsx`
**Effort:** 45 minutes

Add a `generationRef` counter to `buildContext()`. Increment on scenario switch. All ctx callbacks check generation before calling setState.

---

### S13. Thread pause through `showCmd`

**Refs:** P3, 03-PRESENTER-CONTROL
**Files:** `hooks/useTerminalSession.ts`, `lib/scenarios.ts`
**Effort:** 1.5 hours

Add `paused` ref to `showCmd` options. After command completes but before advancing progress, check pause state. This makes Pause button functional during long commands.

---

### S14. Wire Builder model selection

**Refs:** 05-WORKFLOW-E2E
**Files:** `pages/Builder.tsx`
**Effort:** 20 minutes

Pass `selectedModel` to the roko CLI command: `${getRoko()} run "${text}" --model ${selectedModel}`.

---

## Tier 3: Quality of Life

Nice to have for a polished demo.

### S15. Fix ConnectScreen overlay timing

**Refs:** 04-VISUAL-AUDIT
**Files:** `pages/Landing.tsx` or wherever ConnectScreen is rendered
**Effort:** 30 minutes

Dismiss ConnectScreen as soon as the Three.js scene is loaded, not after the health probe. Or add a fade-in transition so the black overlay doesn't flash.

---

### S16. Share demo constants via module

**Refs:** V3, 04-VISUAL-AUDIT
**Files:** New `lib/demo-constants.ts`, update Landing, CostDashboard, ChainView, Explorer
**Effort:** 30 minutes

```ts
export const DEMO_METRICS = {
  episodes: 847,
  totalCost: 1.42,
  cFactor: 0.847,
  gatePass: 93.1,
  agents: 5,
} as const;
```

---

### S17. Add missing `waitForStep()` calls

**Refs:** P7, P8, 03-PRESENTER-CONTROL
**Files:** `lib/scenarios.ts`
**Effort:** 20 minutes

- `prdResearchLoop`: Add `waitForStep()` before phase 6
- `knowledgeTransfer`: Add `waitForStep()` before first `setupWorkspace`

---

### S18. Always render GateBar container

**Refs:** P10, 03-PRESENTER-CONTROL
**Files:** `pages/Demo.tsx`
**Effort:** 15 minutes

Replace conditional render with always-present container. Show "waiting for gates..." when empty.

---

### S19. Fix `useBench.ts` double-start interval leak

**Refs:** B8, 02-RUNTIME-BUGS
**Files:** `hooks/useBench.ts`
**Effort:** 10 minutes

Add `if (pollRef.current) clearInterval(pollRef.current);` at start of `startRun`.

---

### S20. Remove duplicate share route

**Refs:** B16, A8, 01-ARCHITECTURE-CRITIQUE
**Files:** `main.tsx`
**Effort:** 10 minutes

Remove the unreachable `/dashboard/share/:token` route.

---

### S21. Fix `resolveRoko` TOCTOU race

**Refs:** B9, 02-RUNTIME-BUGS
**Files:** `hooks/useTerminalSession.ts`
**Effort:** 15 minutes

Use a promise-based lock to serialize concurrent calls.

---

### S22. Add per-terminal close button

**Refs:** 05-WORKFLOW-E2E
**Files:** `pages/Terminal.tsx`
**Effort:** 30 minutes

Add X button on each terminal header. Remove single terminal from array, close its WS.

---

### S23. Fix Builder preset overflow

**Refs:** 04-VISUAL-AUDIT
**Files:** `pages/Builder.css`
**Effort:** 5 minutes

Add `flex-wrap: wrap` to `.builder-presets`.

---

### S24. Make server probe re-probe on TTL

**Refs:** B5, 02-RUNTIME-BUGS
**Files:** `hooks/useApiWithFallback.ts`
**Effort:** 20 minutes

Reset `_probePromise` after 60 seconds so server recovery is detected.

---

### S25. Throttle Knowledge Graph animation

**Refs:** 04-VISUAL-AUDIT
**Files:** `pages/dashboard/KnowledgeGraph.tsx`
**Effort:** 20 minutes

Only redraw when simulation energy exceeds threshold. Pause animation when tab not visible.

---

## Effort Estimates

| Tier | Items | Total Effort |
|------|-------|-------------|
| Tier 1 (Must fix) | S1–S5 | ~4 hours |
| Tier 2 (High impact) | S6–S14 | ~9 hours |
| Tier 3 (QoL) | S15–S25 | ~4 hours |
| **Total** | **25 items** | **~17 hours** |

---

## Recommended Execution Order

For a single day of focused work, do Tier 1 + selected Tier 2:

1. **S1** — CSS variables (30 min) — instantly fixes 4 broken pages
2. **S4** — Remove Jobs link (10 min) — removes dead end
3. **S5** — KnowledgeEntries fallback (15 min) — fixes offline experience
4. **S7** — Server health false positive (20 min) — fixes demo start confusion
5. **S8** — ChainView leak (15 min) — fixes memory leak
6. **S2** — Speed control (2 hr) — biggest presenter-experience win
7. **S3** — Step mode for race/providers/explore (1 hr) — step mode works everywhere
8. **S6** — WS listener accumulation (45 min) — fixes critical runtime bug
9. **S10** — Empty states (2 hr) — eliminates dark voids
10. **S12** — Stale callback guard (45 min) — prevents scenario bleed

That's ~8 hours for the 10 highest-impact fixes, covering all Tier 1 and the most impactful Tier 2 items.
