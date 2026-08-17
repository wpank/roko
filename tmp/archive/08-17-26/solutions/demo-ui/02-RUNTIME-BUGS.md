# Runtime Bugs

Bugs that crash, leak, or produce wrong results at runtime.

---

## Critical (Crash / Data Corruption)

### B1. WebSocket listener accumulation on reconnect

**File:** `hooks/useTerminal.ts:212–222`

`connectWs()` registers `term.onData()` and `term.onResize()` inside each call. `connectWs()` is called on initial mount AND on every reconnection (every 2s when disconnected). After N reconnects, N copies of each listener exist. Every keypress sends N duplicate messages to the server.

The `IDisposable` objects returned by xterm's `onData`/`onResize` are never stored or disposed.

**Impact:** Duplicate keystrokes sent to PTY after any reconnection event. User sees double/triple typed characters.

**Fix:** Register `onData`/`onResize` once outside `connectWs()`. Use `wsRef.current` inside the callback so reconnection just swaps the WS reference.

---

### B2. `Promise.all` in race/providerRace scenarios lacks null guards

**File:** `lib/scenarios.ts:635–648, 962, 1072–1101`

The `race` scenario fires two `showCmd` calls in `Promise.all`. The `providerRace` scenario maps over `entries` array. If `entries[i]` is null (scenario pane count mismatch), accessing `.outputBuffer` crashes:

```ts
const buf = stripAnsi(e.outputBuffer); // e could be null
```

The unhandled rejection leaves `isRunning = true` permanently until page reload.

**Impact:** Crash. Demo requires page reload to recover.

**Fix:** Add null guard: `if (!e) return;` before accessing `outputBuffer`. Wrap each inner function in try/catch inside `Promise.all`.

---

### B3. Hardcoded `localhost:8545` breaks all non-local deployments

**File:** `hooks/useChain.ts:67–68, 218–226`

Both the WebSocket URL and JSON-RPC endpoint are hardcoded:
```ts
const MIRAGE_HOST = 'localhost:8545';
```

Also hardcoded in 8+ curl commands baked into scenario strings in `scenarios.ts`.

**Impact:** Total failure of chain features in any deployed environment.

**Fix:** `const MIRAGE_HOST = import.meta.env.VITE_MIRAGE_URL || 'localhost:8545'`

---

## High (Leak / Wrong Result)

### B4. `ws.onerror` sets status to `'connected'` instead of `'disconnected'`

**File:** `hooks/useTerminal.ts:205–210`

```ts
ws.onerror = () => {
  if (!handle.ws) {
    handle.status = 'connected';  // BUG: should be 'disconnected'
    setStatus('connected');
  }
};
```

**Impact:** UI shows terminal as "connected" when it has actually failed. Misleading status during a live demo.

**Fix:** Change to `'disconnected'`.

---

### B5. Server probe runs once and is cached forever

**File:** `hooks/useApiWithFallback.ts:36–50`

```ts
let _serverLive: boolean | null = null;
let _probePromise: Promise<void> | null = null;
```

The probe runs exactly once per page load. If the server comes online after the initial probe, all components continue serving demo data permanently.

**Impact:** Server recovery never detected without full page reload.

**Fix:** Add TTL: re-probe after 60 seconds. Or poll `useServerHealth` instead of singleton probe.

---

### B6. `catch` in `useApiWithFallback.get` swallows all errors silently

**File:** `hooks/useApiWithFallback.ts:62–76`

The `catch` block returns demo fallback data for *all* errors, including malformed JSON, type errors, and programming bugs. A 200 response with bad data silently becomes demo data with no logging.

**Impact:** Data corruption goes undetected. Callers cannot distinguish "server error, fell back" from "server returned good data."

**Fix:** Only fall back on network/HTTP errors (`instanceof TypeError` or `!res.ok`). Rethrow or log JSON parse errors.

---

### B7. `ChainView.tsx` setTimeout/setInterval leak on unmount

**File:** `pages/dashboard/ChainView.tsx:64–89`

The hash typewriter animation uses a `setTimeout` inside a `setInterval` callback. The `setTimeout` return value is never stored. If the component unmounts during the 3-second pause:
1. The cleanup function clears `intervalRef.current` (which is null at that point)
2. 3 seconds later, `setTimeout` fires, calls `setTypedHash('')` on unmounted component
3. Starts a new `setInterval` that leaks forever

**Impact:** Memory leak + stale setState after unmount.

**Fix:** Store timeout in a `timeoutRef` and clear it in cleanup.

---

### B8. `useBench.ts` pollRef interval not cleared on double-start

**File:** `hooks/useBench.ts:231–254`

If `startRun` is called twice before the first run completes, the old polling interval is orphaned. It keeps calling `setActiveRun`/`setHistory` with stale data, overwriting the new run's state.

**Impact:** Stale run data overwrites current run display.

**Fix:** Clear `pollRef.current` at the start of `startRun` before creating a new interval.

---

### B9. `resolveRoko` has TOCTOU race on concurrent calls

**File:** `hooks/useTerminalSession.ts:20–51`

`rokoResolved` and `resolvedRoko` are module-level globals. When multiple panes call `setupWorkspace` concurrently (e.g., `explore` scenario with 4 panes), two calls to `resolveRoko` when `rokoResolved = false` both enter the body, send detection commands to different handles, and the last to finish overwrites the result.

**Impact:** Could use wrong roko binary path on first multi-pane scenario.

**Fix:** Use a promise-based lock: `let _resolving: Promise<string> | null = null;`

---

### B10. `useServerHealth` first-failure reports `'connected'`

**File:** `hooks/useServerHealth.ts:22–27`

```ts
setStatus(checked ? 'disconnected' : 'connected');
```

First health check failure sets status to `'connected'` (intentional demo-mode). This means `Demo.tsx`'s `handlePlay` guard passes, scenario starts, then hits WebSocket failures for ~8 seconds before the fallback kicks in.

**Impact:** Confusing 8-second hang on first play when server is offline.

**Fix:** If demo mode is intentional, add a `demo` status distinct from `connected`. If not, change to `'disconnected'`.

---

### B11. `buildContext` snapshots terminal handles that may be null

**File:** `pages/Demo.tsx:217–262`

`buildContext()` is called in `handlePlay` and snapshots `handleRefs.current`. On the very first play after mount, individual terminal `useEffect` hooks may not have run yet — handles are null. The `filter` removes nulls, so `entries` could be shorter than `scenario.panes`.

**Impact:** Scenario starts with fewer terminals than expected. Commands may be sent to wrong handles.

**Fix:** Add a readiness check: wait for all handles to be non-null before allowing play.

---

## Medium (Cosmetic / Dev Warnings)

### B12. `TerminalPaneWithHandle` useEffect has no dependency array

**File:** `pages/Demo.tsx:667–671`

Runs after every render, continuously overwriting `handleRef.current`. In React 18 Strict Mode, fires twice on mount.

**Fix:** Add `[handle, handleRef]` dependency array.

---

### B13. `timeline.onChange` / `playback.onProgress` callbacks fire after unmount

**File:** `pages/Demo.tsx:166–172`

If Demo unmounts while a scenario is running, `setTimelineSteps` etc. fire on the dead component. React 18 suppresses the warning but it's still wrong.

**Fix:** Guard with `mountedRef` or clear listeners in cleanup return.

---

### B14. Long-running scenario keeps calling setState after component unmount

**File:** `pages/Demo.tsx:341–349`

If user navigates away during a running scenario, all `showCmd` / `setupWorkspace` promises keep resolving and calling `setStats`, `setGates`, `setLogEntries` on the unmounted component.

**Fix:** Thread an `AbortController` through the scenario context. Check `signal.aborted` in all callbacks.

---

### B15. `trackMetrics` intervals in `race` scenario not in try/finally

**File:** `lib/scenarios.ts:626–651`

```ts
const leftTracker = trackMetrics(left, { ... });
const rightTracker = trackMetrics(right, { ... });
await Promise.all([...]);
clearInterval(leftTracker);  // skipped if Promise.all rejects
clearInterval(rightTracker);
```

**Fix:** Wrap in try/finally.

---

### B16. Duplicate `/share/:token` routes

**File:** `main.tsx:38, 48`

Two routes: `/dashboard/share/:token` → `ShareView` and `/share/:token` → `SharePage`. They hit different APIs and render different components. No UI links to the dashboard version.

**Fix:** Remove the unreachable dashboard-level share route, or consolidate.

---

### B17. `useSSE.ts` reconnectTimer uninitialized

**File:** `hooks/useSSE.ts:10`

`let reconnectTimer: ReturnType<typeof setTimeout>;` — no initializer. `clearTimeout(undefined)` is benign but semantically wrong.

**Fix:** Initialize to `undefined` explicitly.

---

## Summary

| ID | Severity | Category | File | Status |
|----|----------|----------|------|--------|
| B1 | Critical | Leak + wrong result | useTerminal.ts | Open |
| B2 | Critical | Crash | scenarios.ts | Open |
| B3 | Critical | Total failure | useChain.ts | Open |
| B4 | High | Wrong result | useTerminal.ts | Open |
| B5 | High | Wrong result | useApiWithFallback.ts | Open |
| B6 | High | Wrong result | useApiWithFallback.ts | Open |
| B7 | High | Leak | ChainView.tsx | **Fixed** (Batch 5) |
| B8 | High | Leak + wrong result | useBench.ts | Open |
| B9 | High | Wrong result | useTerminalSession.ts | Open |
| B10 | High | UX confusion | useServerHealth.ts | **Fixed** (Batch 4) |
| B11 | High | Wrong result | Demo.tsx | Open |
| B12 | Medium | Cosmetic | Demo.tsx | Open |
| B13 | Medium | Dev warning | Demo.tsx | Open |
| B14 | Medium | Leak | Demo.tsx | Open |
| B15 | Medium | Leak | scenarios.ts | Open |
| B16 | Medium | Architecture | main.tsx | **Fixed** (Batch 2) |
| B17 | Medium | Cosmetic | useSSE.ts | Open |

**B18–B38: See [07-DEEP-SOURCE-AUDIT.md](07-DEEP-SOURCE-AUDIT.md) for 21 additional bugs found in the line-by-line source review.**
