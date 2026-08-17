# Deep Source Audit

Additional bugs, UX gaps, performance issues, and dead code found via line-by-line review of every component, hook, and page — not already catalogued in 01-06.

Screenshots: `/tmp/roko-audit-1777443964541/`

---

## New Bugs (not in 02-RUNTIME-BUGS.md)

### B18. KnowledgeGraph canvas glow rendering is dead code

**File:** `pages/dashboard/KnowledgeGraph.tsx:206–211`

The glow fill code does a fragile `.replace()` chain to convert hex→rgba, then immediately overwrites `ctx.fillStyle = 'transparent'`. The glow fill is never painted. The `shadowBlur` on a transparent fill has no visible effect. The entire glow rendering path is dead code — nodes render without glow.

**Impact:** Knowledge graph nodes look flat/dim. The intended glow effect around nodes doesn't render.

**Fix:** Remove the dead `fillStyle = 'transparent'` line and fix the rgba conversion, or use `shadowColor`/`shadowBlur` on the actual node fill (not a transparent fill).

---

### B19. KnowledgeGraph animation loop never stops

**File:** `pages/dashboard/KnowledgeGraph.tsx:227`

`requestAnimationFrame(draw)` is called unconditionally at the end of every `draw()`. The graph redraws at 60fps forever, even after the physics simulation has settled. Unlike AgentFleet (which caps at 120 frames), this loop runs indefinitely.

**Impact:** Continuous CPU/GPU drain. Battery killer on laptops. Canvas repaints 60 times/second with no visual change after ~2 seconds.

**Fix:** Add an energy threshold: if simulation velocity falls below a threshold, stop calling rAF. Resume when data changes. Also use Page Visibility API to pause when tab is backgrounded.

---

### B20. KnowledgeGraph O(n²) tick at 60fps

**File:** `pages/dashboard/KnowledgeGraph.tsx` (tick function)

The repulsion calculation iterates every node pair (O(n²)) inside the tick function, which runs at 60fps. With 50+ nodes this will visibly drop frame rate on modest hardware.

**Fix:** Cap frame rate to 30fps for the physics simulation, or use Barnes-Hut approximation.

---

### B21. GateWaterfall `roundRect` crashes on older Safari

**File:** `components/GateWaterfall.tsx:47`

`ctx.roundRect()` is not available in Safari < 15.4 or older Firefox. On unsupported browsers this throws `TypeError: ctx.roundRect is not a function` and the entire canvas goes blank.

**Fix:** Add a `roundRect` polyfill or fall back to manual arc-based rounded rectangles.

---

### B22. GateWaterfall doesn't clear canvas when `runs` becomes empty

**File:** `components/GateWaterfall.tsx:56`

When `runs.length === 0`, the function returns early without clearing the canvas. If data goes from non-empty to empty (e.g., filter change), the canvas shows stale rendering from the previous data.

**Fix:** Clear the canvas before the early return: `ctx.clearRect(0, 0, w, h)`.

---

### B23. BenchRunDetail "Loading run undefined..."

**File:** `pages/BenchRunDetail.tsx:274`

If the user navigates to `/bench/run/` without an ID, `useParams` returns `id: undefined`, and the loading text shows `Loading run undefined...`. If the ID doesn't match any demo run, loading text displays forever — no "not found" state.

**Fix:** Add a null/undefined check for `id` and a "run not found" error state after the fetch resolves with no data.

---

### B24. BenchRunDetail tokenCost rates are wrong

**File:** `pages/BenchRunDetail.tsx:20–32`

Hardcoded rates don't match actual Anthropic pricing (as of 2026). The haiku rate computes to ~$0.25/MTok but Haiku 3.5 is $0.80/MTok — 3x off. Cost breakdown charts show misleading numbers.

**Fix:** Update rates to current pricing, or mark costs as "estimated" in the UI.

---

### B25. BenchCompare allows selecting the same run for both A and B

**File:** `pages/BenchCompare.tsx:80`

Both selects show the same run list. Selecting run X for both A and B produces a comparison with all-zero deltas. No validation prevents `selectedA === selectedB`.

**Fix:** Filter out `selectedA` from the B dropdown options, and vice versa.

---

### B26. BenchCompare stuck loading when compare endpoint fails

**File:** `pages/BenchCompare.tsx:80`

When the compare endpoint fails, fallback runs from the list may lack full `results` arrays (summary-only). `taskMatrix` stays empty and the comparison pane shows "Loading task comparison data..." forever.

**Fix:** Detect when fallback data lacks `results` and show an error instead of permanent loading.

---

### B27. useSSE reconnect timer leak on rapid errors

**File:** `hooks/useSSE.ts:10`

If `es.onerror` fires multiple times rapidly, `reconnectTimer = setTimeout(connect, 3000)` overwrites the previous timer reference, leaking earlier timers. Cleanup only clears the last one.

**Fix:** `clearTimeout(reconnectTimer)` before assigning a new one.

---

### B28. useSSE `connected` stays true on clean server close

**File:** `hooks/useSSE.ts`

`connected` is set to `true` on `es.onopen` but only set to `false` in `onerror`. A clean server-side close doesn't trigger `onerror`, so `connected` stays `true` while the connection is actually dead.

**Fix:** Also set `connected = false` in `es.onerror` → already done, but also need to handle `readyState === EventSource.CLOSED` on reconnect attempts.

---

### B29. useSSE connects immediately even when server is offline

**File:** `hooks/useSSE.ts`

No guard against starting SSE when server is unreachable. Creates and immediately destroys an `EventSource` every 3 seconds indefinitely in offline mode.

**Fix:** Check `_serverLive` or `useServerHealth` status before opening SSE connection.

---

### B30. useTerminal `reconnectTimer` can leak on rapid error+close

**File:** `hooks/useTerminal.ts`

If `ws.onerror` fires then `ws.onclose` fires (common pattern), two `reconnectTimer = setTimeout(connectWs, 2000)` calls happen. The first timer's reference is lost. Cleanup only clears the last.

**Fix:** `clearTimeout(reconnectTimer)` before each `setTimeout`.

---

### B31. useTerminal `TextDecoder` created per message

**File:** `hooks/useTerminal.ts:200`

`new TextDecoder()` is created inside the binary message handler, which runs on every WebSocket message. Should be created once outside the handler.

**Fix:** Move `const decoder = new TextDecoder()` outside the callback.

---

### B32. useTerminalSession `resolveRoko` race with multi-pane

**File:** `hooks/useTerminalSession.ts:20–51`

Two concurrent calls to `resolveRoko()` from different terminal panes both execute detection commands. The global cache (`resolvedRoko`) is written by whichever finishes last. If terminal B is in a different working directory, the cached path may be wrong for it.

**Fix:** Use a promise-based lock: `let _resolving: Promise<string> | null = null`.

---

### B33. PrdPipelinePanel marks all phases as "done" on failure

**File:** `components/PrdPipelinePanel.tsx`

When `state.phase === 'failed'`, `phaseIndex('failed')` returns 9 (the last index). All visible phases have `phaseStep < 9`, so all render as `'done'`. Phases after the actual failure point should render as `'pending'`, not `'done'`.

**Fix:** Track the failure point separately from the phase index, or use the previous phase as the done-threshold on failure.

---

### B34. Demo.tsx `playback`/`timeline` listeners accumulate on remount

**File:** `pages/Demo.tsx:39–40, 168–175`

`playback` and `timeline` are module-level singletons. The `useEffect` at lines 168–175 registers `onChange`/`onProgress` listeners but has **no cleanup return**. If Demo is unmounted and remounted (navigate away and back), new listeners stack on top of old ones. Each listener calls `setTimelineSteps` etc., causing redundant state updates.

**Fix:** Return cleanup functions from the `useEffect` that remove the listeners. Or lazy-init the singletons inside the component and recreate on mount.

---

### B35. Demo.tsx `ciBlocks` and `ciPositions` are stateful but never mutated

**File:** `pages/Demo.tsx:80–102`

`ciBlocks` and `ciPositions` are declared with `useState` but are never updated after initialization. They should be `const` values outside the component or `useMemo`.

**Fix:** Move to module-level constants.

---

### B36. AgentFleet topology polling is sequential, not parallel

**File:** `pages/dashboard/AgentFleet.tsx:436–441`

Two `get()` calls in `poll()` are `await`ed sequentially. With 5-second poll interval, each cycle wastes ~half the budget on sequential latency.

**Fix:** Use `Promise.all([get('/agents'), get('/topology')])`.

---

### B37. AgentFleet force simulation stops at 120 frames

**File:** `pages/dashboard/AgentFleet.tsx`

The simulation caps at exactly 120 frames (~2 seconds at 60fps). If a node hasn't settled by then, it freezes mid-transition. For larger graphs or slow initial positions, nodes stop moving while still overlapping.

**Fix:** Run until energy falls below a threshold, with a higher frame cap (e.g., 300).

---

### B38. AgentFleet `sameTopology` assumes stable ordering

**File:** `pages/dashboard/AgentFleet.tsx`

Node arrays are compared positionally (by index). If the server returns nodes in a different order between polls, the topology is marked as changed and the physics simulation resets — causing jarring visual jumps even when nothing actually changed.

**Fix:** Sort nodes by ID before comparison, or use a set-based comparison.

---

## New UX Issues

### U1. Explorer has no auto-polling

**File:** `pages/Explorer.tsx`

Explorer is the only data page that doesn't auto-refresh. Data fetched once on mount (or tab switch) and never updated. CostDashboard, AgentFleet, KnowledgeGraph all poll. Stale data on long sessions.

**Fix:** Add `setInterval(refresh, 10_000)` like sibling pages.

---

### U2. Explorer episode search matches field names

**File:** `pages/Explorer.tsx`

Search uses `JSON.stringify(ep).toLowerCase().includes(s)`. Searching "agent" matches every episode because all have an `agent_id` field. Extremely noisy.

**Fix:** Search only across specific fields: `ep.id`, `ep.agent_id`, `ep.task_id`, `ep.output`.

---

### U3. Explorer events/episodes tabs have no empty state

**File:** `pages/Explorer.tsx`

If `filteredEpisodes` or `events` arrays are empty, the container renders as an empty div. No "no results" or "no events recorded yet" message.

**Fix:** Add empty-state messages.

---

### U4. Explorer events use array index as React key

**File:** `pages/Explorer.tsx:240`

`key={i}` on events. If events are prepended (newest first), React reconciliation will be incorrect — each event gets the wrong DOM node.

**Fix:** Use event ID or timestamp as key.

---

### U5. Explorer fabricates provider names from count

**File:** `pages/Explorer.tsx:66–70`

When the API returns `{healthy: 4, total: 5}`, the code hardcodes provider names `['claude', 'openai', 'gemini', 'ollama', 'perplexity']` and marks the last one as down. The "down" provider is always `perplexity` regardless of reality.

**Fix:** Fetch actual provider list from `/api/providers/health`, or show counts without names.

---

### U6. BenchCompare has no "need 2+ runs" empty state

**File:** `pages/BenchCompare.tsx`

If `runs.length < 2`, the dropdowns appear but there's no message explaining you need at least two runs to compare.

**Fix:** Show "Complete at least 2 benchmark runs to compare results" when `runs.length < 2`.

---

### U7. CostDashboard hardcoded fallbacks in live render path

**File:** `pages/dashboard/CostDashboard.tsx`

Inline `?? 791`, `?? 56`, `?? 14523`, `?? '0.9.2'` fallbacks appear in live-data render paths. If the server is live but returns `null` for specific fields, hardcoded numbers silently substitute. User sees demo values mixed with real data.

**Fix:** Show "—" or "unknown" for null fields instead of hardcoded numbers.

---

### U8. CostDashboard `Promise.all` blocks all data on single failure

**File:** `pages/dashboard/CostDashboard.tsx`

Seven parallel `get()` calls in `Promise.all`. One failure means all data stays stale for that cycle. (Mitigated by `useApiWithFallback` which catches individual errors, but if the wrapper itself throws, all data is lost.)

**Fix:** Use `Promise.allSettled` or individual try/catch per call.

---

### U9. CostDashboard `pulse-dot` keyframe injected inline

**File:** `pages/dashboard/CostDashboard.tsx`

`<style>` tag containing `@keyframes pulse-dot` is injected into the DOM on every render. Should be in a CSS file.

**Fix:** Move to rosedust.css or a dashboard-specific CSS file.

---

### U10. No ErrorBoundary around individual dashboard panes

**File:** `pages/dashboard/CostDashboard.tsx`

If any chart component (CostChart, CFactorSparkline, BarChart) throws during render, the entire DashboardLayout crashes. Individual pane-level error boundaries are missing.

**Fix:** Wrap each Pane in a lightweight ErrorBoundary that shows "Chart unavailable" instead of crashing the page.

---

### U11. ConnectScreen component is dead code

**File:** `components/ConnectScreen.tsx`

The component exists but is not rendered anywhere. Demo.tsx checks server health inline and logs to the command log instead of showing ConnectScreen. This is a whole component that serves no purpose.

**Fix:** Delete the file, or integrate it into Demo.tsx if the overlay behavior is desired.

---

### U12. `_serverLive` probe never re-probes

**File:** `hooks/useApiWithFallback.ts:36–37`

The server probe runs exactly once and is cached in a module-level variable. If the server comes online after the initial probe failure, all components continue using demo data for the entire session. Server recovery is never detected without a full page reload.

**Fix:** Add a TTL: re-probe after 30–60 seconds. Or subscribe to `useServerHealth` to detect recovery.

---

### U13. `_seedCount`/`_nonSeedCount` never reset — data mode gets stuck

**File:** `hooks/useApiWithFallback.ts:41–42`

Tally counters only increase. Once any non-seed record is seen, `deriveDataMode()` returns `'live'` forever — even if the server goes offline and all subsequent responses are seed data. The mode is permanently "stuck" at `'live'`.

**Fix:** Reset counters periodically, or use a sliding window instead of cumulative counts.

---

### U14. `useApiWithFallback.post()` returns `{} as T` on failure

**File:** `hooks/useApiWithFallback.ts:136`

Callers expecting a typed response (e.g., `post<{id: string}>()`) silently get `{}` with no `id` field. Any caller that accesses properties without null-checking crashes. `useBench.ts` handles this with `res.id ?? \`demo-${Date.now()}\``, but other callers may not.

**Fix:** Return `null` on failure and force callers to handle it, or throw a typed error.

---

## New Visual Issues

### V6. Builder/Terminal rainbow color bar is visually jarring

**File:** `pages/Builder.tsx`, `pages/Terminal.tsx`

Both pages show xterm terminals that execute `tput colors` on connect, producing a rainbow color strip. Against the dark rosedust theme, this rainbow bar is extremely jarring and looks like a rendering artifact, not a feature.

**Fix:** Remove the initial `tput colors` command from the terminal session setup, or clear the terminal after the probe completes.

---

### V7. Font sizes below 11px throughout the codebase

Multiple files use `fontSize: 8`, `9`, `10` for canvas labels, stat card subtitles, and table headers. These are below the accessibility minimum for readable text on standard DPI displays.

**Affected files:** CostDashboard.tsx, KnowledgeGraph.tsx, StatCard, GateBar, PrdPipelinePanel, Explorer.tsx, and many more.

**Fix:** Minimum 11px for all text. Use 10px only in canvas where zoom is available.

---

### V8. Explorer `position: fixed` left border bleeds across pages

**File:** `pages/Explorer.css`

The animated rose left-border uses `position: fixed` instead of `position: absolute`. This means the 2px rose line stays visible even after navigating away from Explorer, painting on top of other pages until the component unmounts.

**Fix:** Change to `position: absolute` relative to the page container.

---

### V9. Dashboard Layout tab overflow has no scroll indicator

**File:** `pages/dashboard/Layout.tsx`

Tab bar uses `overflowX: auto` but on narrow viewports, the scrollbar appears with browser defaults (ugly on dark themes). No gradient fade or arrow indicator to show more tabs are available.

**Fix:** Add `-webkit-scrollbar: display: none` with scroll-margin, or use gradient fade-out edges.

---

### V10. CostDashboard "Online" status uses italic serif font

**File:** `pages/dashboard/CostDashboard.tsx`

The "Online" status text uses italic serif (Fraunces display) while everything else in the mosaic uses mono. This is an unusual choice that breaks the visual consistency of the status row.

**Fix:** Use `var(--mono)` for the status indicator.

---

## Accessibility Issues

### A1. Episode items in Explorer are keyboard-inaccessible

**File:** `pages/Explorer.tsx:206–228`

Clickable episode rows are `<div>` elements with `onClick` but no `role="button"`, `tabIndex`, or `onKeyDown`. They cannot be reached or activated via keyboard.

**Fix:** Use `<button>` elements, or add `role="button" tabIndex={0} onKeyDown={handleEnter}`.

---

### A2. Terminal columns selector has no label

**File:** `pages/Terminal.tsx`

The 1/2/4 column buttons have no accessible label. Screen readers see three unlabeled buttons.

**Fix:** Add `aria-label="1 column"` etc. to each button.

---

### A3. GateBar status icons have no text alternatives

**File:** `components/GateBar.tsx`

Unicode status characters (✓ ✗ ○ –) serve as the only status indicators. No `aria-label` or `title` on the spans. Screen readers announce the literal Unicode characters.

**Fix:** Add `aria-label="passed"` / `aria-label="failed"` etc.

---

### A4. TopNav logo mark has no accessible text

**File:** `components/TopNav.tsx:54`

`<span className="mark" />` is a decorative element with no text or `aria-label`. The `<b>ROKO</b>` helps, but the mark itself conveys nothing to assistive tech.

**Fix:** Add `aria-hidden="true"` to the mark span.

---

### A5. Dashboard nav has no `aria-label`

**File:** `pages/dashboard/Layout.tsx`

The inner `<nav>` for dashboard tabs has no label. It's indistinguishable from the top nav for screen readers.

**Fix:** Add `aria-label="Dashboard sections"`.

---

## Performance Issues

### PERF1. CostDashboard `useCountUp` animation can jump

**File:** `pages/dashboard/CostDashboard.tsx`

`useCountUp` captures `from` as the current value at effect start, but `val` is deliberately excluded from deps. If the target changes mid-animation, the next animation starts from the previous _target_, not the current interpolated position. This causes visual jumps (number snaps back then counts up again).

**Fix:** Use a ref to track the current displayed value and always animate from that.

---

### PERF2. CostDashboard gate pass rate computed inline, not memoized

**File:** `pages/dashboard/CostDashboard.tsx`

Gate pass rate calculation with hardcoded fallbacks runs in the render path, not in `useMemo`. Recomputes on every render.

**Fix:** Move to `useMemo`.

---

### PERF3. AgentFleet `draw` callback recreated on every `data` change

**File:** `pages/dashboard/AgentFleet.tsx`

`draw` is `useCallback([data])`. Every poll that detects a topology change (even ordering differences) recreates `draw`, which restarts the entire physics simulation from scratch. Visually jarring.

**Fix:** Only restart simulation when actual topology changes (node/edge sets differ, not ordering).

---

## Dead Code

### DC1. ConnectScreen.tsx — never rendered

Defined but not used anywhere. No import in any page.

### DC2. Demo.tsx `ciBlocks` and `ciPositions` — useState but never updated

Should be module-level constants.

### DC3. KnowledgeGraph glow fill code — overwritten immediately

Lines 206–211: fillStyle set then immediately overwritten to transparent.

### DC4. `_progressText` / `_progressLabel` in Demo.tsx

Updated by callbacks but never rendered. Acknowledged with underscore prefix.

### DC5. `useBench.ts` `_sseEvents` — destructured but unused

From `useBenchSSE`, only `lastEvent` and `clear` are used.

### DC6. `screenshot-all.mjs` references `/jobs` route

No such route exists. This is a stale entry that would produce a 404 screenshot.

---

### B39. Explorer Events tab crashes the entire app

**File:** `pages/Explorer.tsx` (Events tab rendering)

Clicking the Events tab on Explorer causes a full crash — the ErrorBoundary catches it and shows "Something went wrong / TRY AGAIN". The nav, top bar, and all content disappear. This is a **complete page crash**, not just an empty state.

**Impact:** Critical. The Events tab is completely non-functional. Clicking it during a demo destroys the entire app state.

**Reproduction:** Navigate to `/explorer`, click the "Events" tab. Instant crash.

**Likely root cause:** The events rendering code likely accesses a property on undefined event objects, or the event data shape from the live API doesn't match the expected type. Since the ErrorBoundary catches it, this is a React render error (not a network error).

**Fix:** Debug the exact render crash (check console in non-headless mode), fix the property access, and add a try/catch in the Events tab rendering or a per-tab ErrorBoundary.

---

### U15. Pages show empty data when server is live but has no content

**File:** `hooks/useApiWithFallback.ts`, all dashboard pages

The fallback system has two modes: server-offline (returns demo data) and server-live (returns real data). But there's a third state: **server-live, endpoint returns empty array**. In this case, the API response is `[]` (valid, non-error), so no fallback triggers. The page shows zeros/empty instead of demo data.

This is the actual reason KnowledgeEntries, CascadeRouter, and other dashboard pages show zeros in dev: `roko serve` is running but has no data in its stores.

**Fix:** Either:
1. Seed `roko serve` with demo data on startup (server-side fix)
2. Add client-side logic: if server is live AND response is empty AND dataMode is still 'unknown', use demo data as placeholder with "seed data" indicator
3. Show a more descriptive empty state: "No knowledge entries yet — run `roko plan run` to generate data"

---

## Summary

| Category | Count | New items |
|----------|-------|-----------|
| Bugs | B18–B39 | 22 new |
| UX Issues | U1–U15 | 15 new |
| Visual Issues | V6–V10 | 5 new |
| Accessibility | A1–A5 | 5 new |
| Performance | PERF1–PERF3 | 3 new |
| Dead Code | DC1–DC6 | 6 new |
| **Total** | | **56 new findings** |

Combined with 02-RUNTIME-BUGS (17 items), the total is **73 catalogued issues**.

**Critical discovery:** B39 — Explorer Events tab crashes the entire app with "Something went wrong". This is a live crash visible to any user who clicks the Events tab.
