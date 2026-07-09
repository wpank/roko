# Master Implementation Checklist

Every open issue across the demo app, organized by system. Each item is a self-contained implementation plan with full file paths, code context, and the proper from-scratch solution.

**Codebase:** `/Users/will/dev/nunchi/roko/roko/demo/demo-app/`
**Stack:** React 19.1, React Router 7.6, Vite 6.x, xterm.js, Three.js, Canvas 2D
**Design system:** ROSEDUST — CSS custom properties in `src/styles/rosedust.css`, dark theme with rose/bone/dream accents

---

## 1. Demo Playback Engine

The demo playback system is the core of the investor presentation. Currently it's built from disconnected pieces: a `PlaybackController` singleton, a `TimelineStepper` singleton, module-level `rawSleep()`, scattered `pausedRef`/`runningRef`/`speedRef` state, and 15 scenarios that each implement their own (mostly broken) control flow. The proper design is a unified `DemoController` class that owns all timing, pause, step, and speed behavior.

### 1.1 DemoController Class

- [ ] **Create `src/lib/demo-controller.ts` — unified playback controller**

  **Context:** Currently three separate objects manage demo state:
  - `PlaybackController` (class in `src/lib/playback.ts`) — owns `waitForStep()` / `advanceStep()`, has dead auto-mode code
  - `TimelineStepper` (class in `src/lib/timeline.ts`) — owns step labels and progress callbacks
  - Module-level `globalSpeed` in `src/hooks/useTerminalSession.ts` — typing speed only

  Plus three refs in `Demo.tsx`: `pausedRef`, `runningRef`, `speedRef`. Scenarios receive all of these through a 16-field `ScenarioContext` interface.

  **What the proper version looks like:**

  Create a single `DemoController` class that absorbs `PlaybackController`, `TimelineStepper`, and the speed/pause/running refs:

  ```typescript
  // src/lib/demo-controller.ts
  export class DemoController {
    private _speed = 1;
    private _paused = false;
    private _running = false;
    private _stepResolve: (() => void) | null = null;
    private _steps: { label: string; description?: string }[] = [];
    private _currentStep = 0;
    private _listeners: Set<(state: DemoState) => void> = new Set();

    get speed() { return this._speed; }
    set speed(v: number) { this._speed = v; this.notify(); }

    get paused() { return this._paused; }
    set paused(v: boolean) { this._paused = v; if (!v && this._stepResolve) { /* don't auto-advance */ } this.notify(); }

    get running() { return this._running; }
    set running(v: boolean) { this._running = v; this.notify(); }

    /** Speed-aware, pause-aware, abort-aware sleep */
    async sleep(ms: number): Promise<void> {
      const end = Date.now() + ms / this._speed;
      while (Date.now() < end) {
        if (!this._running) return;
        while (this._paused && this._running) {
          await new Promise(r => setTimeout(r, 50));
        }
        if (!this._running) return;
        await new Promise(r => setTimeout(r, Math.min(50, end - Date.now())));
      }
    }

    /** Block until presenter clicks Next */
    async waitForStep(label?: string): Promise<void> {
      if (!this._running) return;
      if (label) { this._currentStep++; this.notify(); }
      return new Promise<void>(resolve => {
        this._stepResolve = resolve;
      });
    }

    /** Presenter clicked Next */
    advanceStep(): void {
      if (this._stepResolve) {
        const resolve = this._stepResolve;
        this._stepResolve = null;
        resolve();
      }
    }

    /** Set timeline steps for current scenario */
    setSteps(steps: { label: string; description?: string }[]): void {
      this._steps = steps;
      this._currentStep = 0;
      this.notify();
    }

    reset(): void {
      this._running = false;
      this._paused = false;
      this._stepResolve = null;
      this._currentStep = 0;
      this._steps = [];
      this.notify();
    }

    subscribe(fn: (state: DemoState) => void): () => void {
      this._listeners.add(fn);
      return () => this._listeners.delete(fn);
    }

    private notify(): void {
      const state: DemoState = {
        speed: this._speed,
        paused: this._paused,
        running: this._running,
        currentStep: this._currentStep,
        totalSteps: this._steps.length,
        steps: this._steps,
      };
      this._listeners.forEach(fn => fn(state));
    }
  }

  export interface DemoState {
    speed: number;
    paused: boolean;
    running: boolean;
    currentStep: number;
    totalSteps: number;
    steps: { label: string; description?: string }[];
  }
  ```

  **Files to modify after creating this:**
  - `src/lib/playback.ts` — delete (absorbed into DemoController)
  - `src/lib/timeline.ts` — delete (absorbed into DemoController)
  - `src/pages/Demo.tsx` — replace `playback`/`timeline` singletons + `pausedRef`/`runningRef`/`speedRef` with single `DemoController` instance
  - `src/hooks/useTerminalSession.ts` — remove `globalSpeed` module variable, read speed from controller
  - `src/lib/scenarios.ts` — all 15 scenarios receive `DemoController` instead of scattered refs

  **Verification:** Speed button changes scenario pacing. Pause halts mid-sleep. Next advances past `waitForStep()`. Reset stops everything immediately.

### 1.2 Interruptible Sleep

- [ ] **Replace all `rawSleep()` calls with `controller.sleep()`**

  **Context:** `rawSleep` is defined in `src/hooks/useTerminalSession.ts:~line 10`:
  ```typescript
  function rawSleep(ms: number): Promise<void> {
    return new Promise(r => setTimeout(r, ms));
  }
  ```
  It's called from:
  - `src/lib/scenarios.ts` — every scenario uses `rawSleep()` for pacing (300ms, 500ms, 800ms, 2000ms, 3000ms delays)
  - `src/hooks/useTerminalSession.ts` — `waitForOpen()`, `waitForPrompt()`, `waitForMarker()` use it for polling
  - `src/hooks/useTerminal.ts` — `typeCmd()` uses `sleep()` (which wraps `rawSleep`) for character delay

  **What to do:**
  1. In `scenarios.ts`: replace every `await rawSleep(N)` with `await ctx.controller.sleep(N)` where `ctx` is the scenario context
  2. In `useTerminalSession.ts`: for internal polling (`waitForOpen`, `waitForPrompt`, `waitForMarker`), keep raw `setTimeout` since these are not user-facing timing — they're polling loops waiting for terminal readiness
  3. In `useTerminal.ts` `typeCmd()`: replace `await sleep(12 + Math.random() * 6)` with a speed-aware delay: `await sleep((12 + Math.random() * 6) / controller.speed)`

  **Grep to find all usages:** `grep -rn 'rawSleep\|await sleep(' src/ --include='*.ts' --include='*.tsx'`

  **Verification:** Set speed to 4x. Run any scenario. Delays should be ~4x shorter. Set speed to 0.5x. Delays should be ~2x longer.

### 1.3 Scenario Context Simplification

- [ ] **Replace 16-field `ScenarioContext` with composable interfaces**

  **Context:** `ScenarioContext` is defined in `src/lib/scenarios.ts` (near the top). It carries:
  ```
  entries, playback, timeline, setMetric, setGate, logCommand, running, paused, speed,
  setPipelinePhase, setPipelineTaskStatuses, setPipelineStream, setPipelineCosts, pipelineExample
  ```
  Every scenario receives all fields even if it uses 3.

  **What the proper version looks like:**
  ```typescript
  // Core — every scenario gets this
  interface ScenarioContext {
    entries: TerminalHandle[];
    controller: DemoController;   // replaces playback + timeline + running + paused + speed
    log: (cmd: string, desc: string) => void;
    cleanup: ScenarioCleanup;     // see item 1.5
  }

  // Extended — scenarios that show metrics/gates
  interface MetricScenarioContext extends ScenarioContext {
    setMetric: (id: string, value: string) => void;
    setGate: (name: string, status: 'pass' | 'fail' | 'pending') => void;
  }

  // Pipeline — scenarios that drive the PRD pipeline visualization
  interface PipelineScenarioContext extends MetricScenarioContext {
    setPipelinePhase: (phase: string) => void;
    setPipelineTaskStatuses: (statuses: TaskStatus[]) => void;
    setPipelineStream: (stream: Partial<StreamState>) => void;
    setPipelineCosts: (costs: CostState) => void;
    pipelineExample: PipelineExample;
  }
  ```

  **Files to modify:**
  - `src/lib/scenarios.ts` — change `ScenarioContext` type, update all 15 scenario function signatures
  - `src/pages/Demo.tsx` — `buildContext()` function constructs the simplified context

### 1.4 Step Mode Coverage

- [ ] **Add `waitForStep()` to all scenarios that lack it**

  **Context:** Only 3 of 15 scenarios fully support step mode. The step mode coverage matrix (from `03-PRESENTER-CONTROL.md`):

  | Scenario | Has waitForStep | Status |
  |----------|----------------|--------|
  | `race` | 0 calls | **Needs fix** |
  | `providers` | 0 calls | **Needs fix** |
  | `explore` | 0 calls (has pause guards) | **Needs fix** |
  | `prdResearchLoop` | 7 (missing phase 6) | **Needs fix** |
  | `knowledgeTransfer` | 4 (missing intro) | **Needs fix** |
  | `prdPipeline` | 5 | OK but no pause guards |
  | `selfhost` | 5 | OK but no pause guards |
  | `providerRace` | 2 | Partial |
  | `knowledgeAccumulation` | 6 | Partial |
  | `chat` | 3 | Partial |
  | `gateRetry` | 6 | **Working** |
  | `dreamConsolidation` | 7 | **Working** |
  | `chainIntelligence` | 6 | **Working** |
  | `mirage` | 0 (instant) | N/A |
  | `builder` | 0 (external) | N/A |

  **What to do:** For each scenario listed as "Needs fix" or "Partial" in `src/lib/scenarios.ts`:
  1. Add `await ctx.controller.waitForStep('Step label')` before major phase transitions
  2. The `race` scenario: add step before `Promise.all`, after both commands finish
  3. The `providers` scenario: add step before `Promise.all`, after all 4 commands finish
  4. The `explore` scenario: add step before parallel pane setup, between command groups
  5. The `prdResearchLoop` scenario: add step before phase 6 (gate results processing)
  6. The `knowledgeTransfer` scenario: add step before first `setupWorkspace` call (intro)
  7. For scenarios marked "Partial": add step gates at any phase transitions that currently auto-advance

  **Verification:** Set playback to step mode. Every scenario should pause at each major phase transition and require pressing Next to continue.

### 1.5 Scenario Cleanup Registry

- [ ] **Create `ScenarioCleanup` class and wire into all scenarios**

  **Context:** Multiple scenarios create `setInterval` timers via `trackMetrics()` and `setInterval()` directly. These leak if:
  - The scenario throws an error
  - The presenter presses Reset mid-scenario
  - The user navigates away from the Demo page

  Affected scenarios and their leaky resources (all in `src/lib/scenarios.ts`):
  - `race`: `leftTracker`, `rightTracker` intervals (lines ~635-651)
  - `gateRetry`: `tracker` interval
  - `providerRace`: multiple `trackers` intervals
  - `knowledgeAccumulation`: `tracker` interval
  - `knowledgeTransfer`: `tracker1`, `tracker2` intervals
  - `chainIntelligence`: `tracker` interval
  - `dreamConsolidation`: `tracker` interval

  **What the proper version looks like:**
  ```typescript
  // src/lib/scenario-cleanup.ts
  export class ScenarioCleanup {
    private cleanups: (() => void)[] = [];

    trackInterval(id: ReturnType<typeof setInterval>): void {
      this.cleanups.push(() => clearInterval(id));
    }

    trackTimeout(id: ReturnType<typeof setTimeout>): void {
      this.cleanups.push(() => clearTimeout(id));
    }

    onDispose(fn: () => void): void {
      this.cleanups.push(fn);
    }

    dispose(): void {
      this.cleanups.forEach(fn => fn());
      this.cleanups = [];
    }
  }
  ```

  **How to wire it:**
  1. Create cleanup instance per scenario run in `Demo.tsx` `handlePlay()`
  2. Pass it through `ScenarioContext`
  3. In each scenario, replace `const tracker = trackMetrics(...)` with `ctx.cleanup.trackInterval(trackMetrics(...))`
  4. In `handleReset()` and the `finally` block of `handlePlay()`, call `cleanup.dispose()`

  **Verification:** Start a scenario with trackers, press Reset mid-run. Verify via browser DevTools that no intervals continue firing.

### 1.6 Stale Callback Guard

- [ ] **Add generation counter to prevent scenario cross-contamination**

  **Context:** In `src/pages/Demo.tsx`, `buildContext()` creates a `ScenarioContext` whose callbacks (`setMetric`, `setGate`, `logCommand`, `setPipelinePhase`, etc.) are closures over the current component's `setState` setters. When the user switches scenarios while one is still running, the old scenario's in-flight promises keep resolving and calling these setters, polluting the new scenario's display.

  **What the proper version looks like:**

  In `Demo.tsx`:
  ```typescript
  const generationRef = useRef(0);

  function buildContext(): ScenarioContext {
    const gen = generationRef.current;
    const guard = <T extends (...args: any[]) => any>(fn: T): T => {
      return ((...args: any[]) => {
        if (generationRef.current !== gen) return; // stale — discard
        return fn(...args);
      }) as T;
    };

    return {
      entries: /* ... */,
      controller: demoController,
      setMetric: guard((id, value) => setStats(prev => ({ ...prev, [id]: value }))),
      setGate: guard((name, status) => setGates(prev => /* ... */)),
      logCommand: guard((cmd, desc) => setLogEntries(prev => /* ... */)),
      // ... all other state-setting callbacks wrapped in guard()
    };
  }
  ```

  In the scenario switch handler: `generationRef.current++` before starting new scenario.

  **Verification:** Start `prdPipeline` scenario. While it's running, switch to `race` scenario. The race sidebar should not show pipeline phases or stale gate results.

---

## 2. Terminal System

The terminal subsystem (`useTerminal.ts`, `useTerminalSession.ts`, `Terminal.tsx`) has listener accumulation bugs, reconnect timer leaks, and architectural issues with how WebSocket lifecycle interacts with xterm event registration.

### 2.1 WebSocket Listener Separation

- [ ] **Fix B1: Separate xterm event listeners from WebSocket lifecycle**

  **Context:** `src/hooks/useTerminal.ts` — the `connectWs()` function (around line 212) registers `term.onData()` and `term.onResize()` listeners *inside* the connection function. `connectWs()` is called on initial mount AND on every reconnection (every 2s when disconnected). After N reconnects, N copies of each listener exist. Every keypress sends N duplicate messages to the server.

  The `IDisposable` objects returned by xterm's `onData`/`onResize` are never stored or disposed.

  **What the proper version looks like:**

  Register xterm listeners ONCE when the terminal instance is created, outside `connectWs()`. Use `wsRef.current` inside the callbacks so reconnection just swaps the WS reference:

  ```typescript
  // In the useEffect that creates the terminal:
  const term = new Terminal(opts);
  const wsRef = useRef<WebSocket | null>(null);

  // Register once:
  const dataDisposable = term.onData(data => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(data);
    }
  });
  const resizeDisposable = term.onResize(size => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify({ type: 'resize', cols: size.cols, rows: size.rows }));
    }
  });

  // connectWs just manages the WebSocket, no listener registration:
  function connectWs() {
    const ws = new WebSocket(url);
    ws.binaryType = 'arraybuffer';
    ws.onopen = () => { setStatus('connected'); };
    ws.onmessage = (e) => { term.write(/* decode e.data */); };
    ws.onclose = () => { setStatus('disconnected'); reconnect(); };
    ws.onerror = () => { setStatus('disconnected'); }; // B4 fix included
    wsRef.current = ws;
  }

  // Cleanup:
  return () => {
    dataDisposable.dispose();
    resizeDisposable.dispose();
    wsRef.current?.close();
  };
  ```

  **Also fixes B4:** Change `ws.onerror` to set status to `'disconnected'` instead of `'connected'` (currently wrong on line ~207).

  **Files:** `src/hooks/useTerminal.ts`

  **Verification:** Open terminal, kill the server, wait for 5+ reconnect cycles, bring server back. Type characters — each should appear exactly once, not duplicated.

### 2.2 Reconnect Timer Leak Fix

- [ ] **Fix B30: Clear reconnect timer before reassigning in useTerminal**

  **Context:** `src/hooks/useTerminal.ts` — if `ws.onerror` fires then `ws.onclose` fires (common browser pattern), two `reconnectTimer = setTimeout(connectWs, 2000)` calls happen. The first timer's reference is lost.

  **What to do:** Add `clearTimeout(reconnectTimer)` before each `setTimeout(connectWs, ...)` in both `onerror` and `onclose` handlers.

  **File:** `src/hooks/useTerminal.ts`

### 2.3 TextDecoder Allocation

- [ ] **Fix B31: Move TextDecoder creation outside message handler**

  **Context:** `src/hooks/useTerminal.ts:~200` — `new TextDecoder()` is created inside the WebSocket binary message handler, which runs on every message.

  **What to do:** Move `const decoder = new TextDecoder()` to the top of the `useEffect` or module scope. Reference it in the handler.

  **File:** `src/hooks/useTerminal.ts`

### 2.4 resolveRoko Race Condition

- [ ] **Fix B9/B32: Add promise-based lock to resolveRoko**

  **Context:** `src/hooks/useTerminalSession.ts:20-51` — `rokoResolved` and `resolvedRoko` are module-level globals. When multiple panes call `setupWorkspace` concurrently (e.g., `explore` scenario with 4 panes), two calls to `resolveRoko` when `rokoResolved = false` both enter the body, send detection commands to different handles, and the last to finish overwrites the result.

  **What the proper version looks like:**
  ```typescript
  let _resolving: Promise<string> | null = null;

  async function resolveRoko(handle: TerminalHandle): Promise<string> {
    if (resolvedRoko) return resolvedRoko;
    if (_resolving) return _resolving;

    _resolving = (async () => {
      // existing detection logic
      const result = /* ... */;
      resolvedRoko = result;
      return result;
    })();

    try {
      return await _resolving;
    } finally {
      _resolving = null;
    }
  }
  ```

  **File:** `src/hooks/useTerminalSession.ts`

### 2.5 Terminal Pane useEffect Dependency

- [ ] **Fix B12: Add dependency array to TerminalPaneWithHandle useEffect**

  **Context:** `src/pages/Demo.tsx:667-671` — the `useEffect` that writes the handle into `handleRef.current` has no dependency array. It runs after every render.

  **What to do:** Add `[handle, handleRef]` as the dependency array.

  **File:** `src/pages/Demo.tsx`

### 2.6 Rainbow Color Bar Removal

- [ ] **Remove `tput colors` rainbow bar from terminal sessions**

  **Context:** Both `src/pages/Builder.tsx` and `src/pages/Terminal.tsx` show xterm terminals that execute `tput colors` on connect, producing a rainbow color strip. Against the dark rosedust theme, this looks like a rendering artifact.

  **What to do:** Find where the initial `tput colors` command is sent (likely in `src/hooks/useTerminalSession.ts` `setupWorkspace()` or in the server-side PTY setup) and remove it. If it's needed for terminal capability detection, run it and then clear the terminal: `term.clear()` after the probe completes.

  **Files:** `src/hooks/useTerminalSession.ts`, possibly server-side PTY code

---

## 3. Data Layer

The data-fetching layer (`useApiWithFallback`, `useServerHealth`, `useSSE`) has issues with probe caching, error swallowing, data mode detection, and SSE connection management.

### 3.1 Server Probe TTL

- [ ] **Fix B5/U12: Add re-probe TTL to useApiWithFallback**

  **Context:** `src/hooks/useApiWithFallback.ts:36-50` — `_serverLive` and `_probePromise` are module-level singletons. The probe runs exactly once per page load. If the server comes online after the initial probe, all components continue serving demo data permanently.

  **What the proper version looks like:**
  ```typescript
  let _serverLive: boolean | null = null;
  let _probePromise: Promise<void> | null = null;
  let _probeTimestamp = 0;
  const PROBE_TTL_MS = 30_000; // re-probe every 30 seconds

  async function probeServer(): Promise<boolean> {
    const now = Date.now();
    if (_serverLive !== null && now - _probeTimestamp < PROBE_TTL_MS) {
      return _serverLive;
    }
    if (_probePromise && now - _probeTimestamp < PROBE_TTL_MS) {
      await _probePromise;
      return _serverLive!;
    }

    _probeTimestamp = now;
    _probePromise = (async () => {
      try {
        const res = await fetch(`${SERVE_URL}/api/health`, { signal: AbortSignal.timeout(3000) });
        _serverLive = res.ok;
      } catch {
        _serverLive = false;
      }
    })();

    await _probePromise;
    _probePromise = null;
    return _serverLive!;
  }
  ```

  **File:** `src/hooks/useApiWithFallback.ts`

### 3.2 Error Discrimination in Fallback

- [ ] **Fix B6: Only fall back on network/HTTP errors, not all errors**

  **Context:** `src/hooks/useApiWithFallback.ts:62-76` — the `catch` block returns demo fallback data for *all* errors, including malformed JSON, type errors, and programming bugs.

  **What the proper version looks like:**
  ```typescript
  async function get<T>(path: string): Promise<T> {
    const live = await probeServer();
    if (!live) return getFallback<T>(path);

    try {
      const res = await fetch(`${SERVE_URL}${path}`);
      if (!res.ok) return getFallback<T>(path); // HTTP error — fall back
      return await res.json() as T;
    } catch (err) {
      if (err instanceof TypeError) {
        // Network error (fetch failed) — fall back
        return getFallback<T>(path);
      }
      // JSON parse error or other programming bug — rethrow
      throw err;
    }
  }
  ```

  **File:** `src/hooks/useApiWithFallback.ts`

### 3.3 Data Mode Sliding Window

- [ ] **Fix U13: Replace cumulative counters with sliding window for data mode**

  **Context:** `src/hooks/useApiWithFallback.ts:41-42` — `_seedCount` and `_nonSeedCount` only increase. Once any non-seed record is seen, `deriveDataMode()` returns `'live'` forever — even if the server goes offline.

  **What the proper version looks like:**
  ```typescript
  const DATA_MODE_WINDOW = 20; // last 20 responses
  const _recentResults: boolean[] = []; // true = seed, false = live

  function recordResult(isSeed: boolean): void {
    _recentResults.push(isSeed);
    if (_recentResults.length > DATA_MODE_WINDOW) _recentResults.shift();
  }

  function deriveDataMode(): 'seed' | 'live' | 'mixed' | 'unknown' {
    if (_recentResults.length === 0) return 'unknown';
    const seedCount = _recentResults.filter(Boolean).length;
    const ratio = seedCount / _recentResults.length;
    if (ratio > 0.8) return 'seed';
    if (ratio < 0.2) return 'live';
    return 'mixed';
  }
  ```

  **File:** `src/hooks/useApiWithFallback.ts`

### 3.4 Post Failure Return Type

- [ ] **Fix U14: Return null instead of `{} as T` on post failure**

  **Context:** `src/hooks/useApiWithFallback.ts:136` — `post()` returns `{} as T` on failure. Callers expecting typed responses silently get empty objects.

  **What to do:** Change return type to `T | null`. Update all callers to handle `null`. Currently only `useBench.ts` calls `post()` — it already has `res.id ?? \`demo-${Date.now()}\`` which handles the null case.

  **File:** `src/hooks/useApiWithFallback.ts`, `src/hooks/useBench.ts`

### 3.5 SSE Connection Guard

- [ ] **Fix B28/B29: Guard SSE connections against offline state and clean close**

  **Context:** `src/hooks/useSSE.ts` — three issues:
  1. B28: `connected` stays `true` on clean server-side close (no `onerror` fires)
  2. B29: SSE connects immediately even when server is offline, creating and destroying EventSource every 3 seconds
  3. B27 was already fixed (clearTimeout before reassigning)

  **What the proper version looks like:**
  ```typescript
  export function useSSE(path: string, { enabled = true }: { enabled?: boolean } = {}) {
    const [connected, setConnected] = useState(false);
    const [lastEvent, setLastEvent] = useState<unknown>(null);
    const esRef = useRef<EventSource | null>(null);

    useEffect(() => {
      if (!enabled) {
        setConnected(false);
        return;
      }

      let reconnectTimer: ReturnType<typeof setTimeout> | undefined;
      let disposed = false;

      function connect() {
        if (disposed) return;
        const es = new EventSource(`${SERVE_URL}${path}`);
        esRef.current = es;

        es.onopen = () => { if (!disposed) setConnected(true); };

        es.onmessage = (e) => {
          if (disposed) return;
          try { setLastEvent(JSON.parse(e.data)); }
          catch { setLastEvent(e.data); }
        };

        es.onerror = () => {
          if (disposed) return;
          setConnected(false);
          es.close();
          clearTimeout(reconnectTimer);
          reconnectTimer = setTimeout(connect, 3_000);
        };

        // Handle clean close (readyState === CLOSED without onerror)
        const checkClosed = setInterval(() => {
          if (es.readyState === EventSource.CLOSED && !disposed) {
            setConnected(false);
            clearInterval(checkClosed);
          }
        }, 1_000);
      }

      connect();

      return () => {
        disposed = true;
        clearTimeout(reconnectTimer);
        esRef.current?.close();
      };
    }, [path, enabled]);

    return { connected, lastEvent };
  }
  ```

  **File:** `src/hooks/useSSE.ts`

### 3.6 CostDashboard Promise.allSettled

- [ ] **Fix U8: Use Promise.allSettled for parallel data fetching in CostDashboard**

  **Context:** `src/pages/dashboard/CostDashboard.tsx` — seven parallel `get()` calls in `Promise.all`. One failure means all data stays stale.

  **What to do:** Replace `Promise.all([...])` with `Promise.allSettled([...])` and extract values from fulfilled results:
  ```typescript
  const results = await Promise.allSettled([
    get<Stats>('/api/stats'),
    get<CostData>('/api/costs'),
    // ... etc
  ]);

  const [stats, costs, ...rest] = results.map(r =>
    r.status === 'fulfilled' ? r.value : null
  );

  if (stats) setStats(stats);
  if (costs) setCosts(costs);
  // ... etc
  ```

  **File:** `src/pages/dashboard/CostDashboard.tsx`

### 3.7 CostDashboard Hardcoded Fallbacks

- [ ] **Fix U7: Replace hardcoded numeric fallbacks with "unknown" indicators**

  **Context:** `src/pages/dashboard/CostDashboard.tsx` — inline `?? 791`, `?? 56`, `?? 14523`, `?? '0.9.2'` fallbacks in render paths. If the server is live but returns `null` for specific fields, hardcoded numbers substitute silently.

  **What to do:** Replace numeric fallbacks with either:
  - "—" for display fields
  - `0` for numeric calculations where a missing value should contribute nothing
  - Keep the demo data system for when the server is fully offline, but don't mix demo numbers into live data

  **File:** `src/pages/dashboard/CostDashboard.tsx`

---

## 4. Demo Page Architecture

The Demo page (`src/pages/Demo.tsx`) is the most complex component. Beyond the playback engine (section 1), it has issues with handle readiness, unmount cleanup, and dead code.

### 4.1 Handle Readiness Check

- [ ] **Fix B11: Wait for all terminal handles before starting scenario**

  **Context:** `src/pages/Demo.tsx:217-262` — `buildContext()` snapshots `handleRefs.current` on play. On first play after mount, individual terminal `useEffect` hooks may not have run — handles are null. The filter removes nulls, so `entries` could be shorter than `scenario.panes`.

  **What the proper version looks like:**
  ```typescript
  async function handlePlay() {
    // Wait for all handles to be ready (max 2 seconds)
    const maxWait = Date.now() + 2000;
    while (Date.now() < maxWait) {
      const handles = handleRefs.current.map(ref => ref.current).filter(Boolean);
      if (handles.length === selectedScenario.panes) break;
      await new Promise(r => setTimeout(r, 50));
    }
    const ctx = buildContext();
    if (ctx.entries.length < selectedScenario.panes) {
      ctx.log('ERROR', `Only ${ctx.entries.length}/${selectedScenario.panes} terminals ready`);
      return;
    }
    // proceed with scenario
  }
  ```

  **File:** `src/pages/Demo.tsx`

### 4.2 Unmount Cleanup

- [ ] **Fix B13/B14: Thread AbortController through scenario execution**

  **Context:** `src/pages/Demo.tsx:166-172, 341-349` — if Demo unmounts while a scenario is running, all `setTimelineSteps`, `setStats`, `setGates`, `setLogEntries` calls fire on the dead component. The `playback.onChange`/`onProgress` listeners accumulate on remount (B34).

  **What the proper version looks like:**
  ```typescript
  useEffect(() => {
    const abortController = new AbortController();
    abortControllerRef.current = abortController;

    // Subscribe to controller state changes
    const unsubscribe = demoController.subscribe((state) => {
      if (abortController.signal.aborted) return;
      setDemoState(state);
    });

    return () => {
      abortController.abort();
      unsubscribe();
      demoController.reset();
    };
  }, []);
  ```

  In `handlePlay()`, pass `abortControllerRef.current.signal` through the scenario context. All setState callbacks check `signal.aborted` before updating.

  **File:** `src/pages/Demo.tsx`

### 4.3 Dead Code Removal

- [ ] **Fix B35, DC2, DC4, DC5: Remove dead state and unused imports**

  **Context:**
  - B35/DC2: `ciBlocks` and `ciPositions` in `src/pages/Demo.tsx:80-102` are `useState` but never updated. Should be module-level constants or `useMemo`.
  - DC4: `_progressText` and `_progressLabel` are set by callbacks but never rendered. Delete them.
  - DC5: `_sseEvents` in `src/hooks/useBench.ts:80` is destructured from `useBenchSSE` but unused. Remove the destructuring.

  **What to do:**
  1. In `Demo.tsx`: Replace `const [ciBlocks] = useState(...)` with `const ciBlocks = useMemo(() => ..., [])` or move to module scope
  2. In `Demo.tsx`: Delete `_progressText`, `_progressLabel` state declarations and their callback assignments
  3. In `useBench.ts`: Change `const { lastEvent, events: _sseEvents, clear: clearSSE }` to `const { lastEvent, clear: clearSSE }`

  **Files:** `src/pages/Demo.tsx`, `src/hooks/useBench.ts`

### 4.4 Delete ConnectScreen

- [ ] **Fix DC1/U11: Delete unused ConnectScreen component**

  **Context:** `src/components/ConnectScreen.tsx` exists but is not imported or rendered anywhere. `Demo.tsx` handles server health inline.

  **What to do:** Delete `src/components/ConnectScreen.tsx`. If it has a corresponding CSS file, delete that too.

  **Verification:** `grep -rn 'ConnectScreen' src/` should return zero results after deletion.

---

## 5. Scenario Bugs

Specific bugs within individual scenario implementations in `src/lib/scenarios.ts`.

### 5.1 Promise.all Null Guards

- [ ] **Fix B2: Add null guards to Promise.all in race/providerRace**

  **Context:** `src/lib/scenarios.ts:635-648, 962, 1072-1101` — the `race` scenario fires two `showCmd` calls in `Promise.all`. The `providerRace` scenario maps over `entries` array. If `entries[i]` is null (pane count mismatch), accessing `.outputBuffer` crashes.

  **What to do:**
  ```typescript
  // In race scenario:
  await Promise.all([
    entries[0] ? showCmd(entries[0], ...) : Promise.resolve(),
    entries[1] ? showCmd(entries[1], ...) : Promise.resolve(),
  ]);

  // In providerRace:
  const buf = e ? stripAnsi(e.outputBuffer) : '';
  ```

  **File:** `src/lib/scenarios.ts`

### 5.2 Interval Cleanup in try/finally

- [ ] **Fix B15: Wrap trackMetrics intervals in try/finally**

  **Context:** `src/lib/scenarios.ts:626-651` — in the `race` scenario, `clearInterval(leftTracker)` and `clearInterval(rightTracker)` are called after `Promise.all`. If `Promise.all` rejects, the intervals are never cleared.

  **What to do:**
  ```typescript
  const leftTracker = trackMetrics(left, { ... });
  const rightTracker = trackMetrics(right, { ... });
  try {
    await Promise.all([...]);
  } finally {
    clearInterval(leftTracker);
    clearInterval(rightTracker);
  }
  ```

  Apply the same pattern to all scenarios that use `trackMetrics`. (This is superseded by the ScenarioCleanup class in 1.5, but should be done as an interim fix if 1.5 is deferred.)

  **File:** `src/lib/scenarios.ts`

### 5.3 Scenario Error Handling

- [ ] **Add error handling to all scenarios**

  **Context:** Only `prdPipeline` checks command results. All other scenarios fire `showCmd` and ignore whether it succeeded. Errors are swallowed silently.

  **What the proper version looks like:** Each scenario wrapped in try/catch:
  ```typescript
  async function raceScenario(ctx: ScenarioContext): Promise<void> {
    try {
      // ... scenario steps
    } catch (err) {
      ctx.log('ERROR', err instanceof Error ? err.message : 'Scenario failed');
      ctx.setGate?.('scenario', 'fail');
    }
  }
  ```

  Or create a `runScenario` wrapper:
  ```typescript
  function runScenario(name: string, fn: (ctx: ScenarioContext) => Promise<void>) {
    return async (ctx: ScenarioContext) => {
      try {
        await fn(ctx);
      } catch (err) {
        ctx.log('ERROR', `${name}: ${err instanceof Error ? err.message : 'Unknown error'}`);
      }
    };
  }
  ```

  **File:** `src/lib/scenarios.ts`

---

## 6. Dashboard Pages

Issues with individual dashboard page components under `src/pages/dashboard/`.

### 6.1 PrdPipelinePanel Failure State

- [ ] **Fix B33: Track failure point separately in PrdPipelinePanel**

  **Context:** `src/components/PrdPipelinePanel.tsx` — when `state.phase === 'failed'`, `phaseIndex('failed')` returns 9 (last index). All phases render as `'done'` instead of showing which phase failed.

  **What the proper version looks like:**
  ```typescript
  function getPhaseStatus(phase: string, currentPhase: string, isFailed: boolean): 'done' | 'active' | 'pending' | 'failed' {
    const currentIdx = phaseIndex(currentPhase);
    const thisIdx = phaseIndex(phase);

    if (isFailed) {
      if (thisIdx < currentIdx) return 'done';
      if (thisIdx === currentIdx) return 'failed';
      return 'pending';
    }

    if (thisIdx < currentIdx) return 'done';
    if (thisIdx === currentIdx) return 'active';
    return 'pending';
  }
  ```

  Store the actual failing phase (not 'failed') in state. When status is 'failed', use the last-known phase as the failure point.

  **File:** `src/components/PrdPipelinePanel.tsx`

### 6.2 AgentFleet Polling Parallelism

- [ ] **Fix B36: Use Promise.all for parallel data fetching in AgentFleet**

  **Context:** `src/pages/dashboard/AgentFleet.tsx:436-441` — two `get()` calls in `poll()` are `await`ed sequentially.

  **What to do:**
  ```typescript
  const [agents, topology] = await Promise.all([
    get<Agent[]>('/api/agents'),
    get<Topology>('/api/topology'),
  ]);
  ```

  **File:** `src/pages/dashboard/AgentFleet.tsx`

### 6.3 AgentFleet Topology Comparison

- [ ] **Fix B38: Sort nodes by ID before topology comparison**

  **Context:** `src/pages/dashboard/AgentFleet.tsx` — `sameTopology()` compares node arrays positionally. If the server returns nodes in a different order, the topology is marked as changed and the physics simulation resets, causing jarring visual jumps.

  **What to do:**
  ```typescript
  function sameTopology(a: TopologyData, b: TopologyData): boolean {
    const sortById = (nodes: Node[]) => [...nodes].sort((x, y) => x.id.localeCompare(y.id));
    const aSorted = sortById(a.nodes);
    const bSorted = sortById(b.nodes);
    if (aSorted.length !== bSorted.length) return false;
    return aSorted.every((node, i) => node.id === bSorted[i].id && node.status === bSorted[i].status);
  }
  ```

  **File:** `src/pages/dashboard/AgentFleet.tsx`

### 6.4 Dashboard Tab Scroll Indicator

- [ ] **Fix V9: Hide scrollbar and add gradient fade on dashboard tab overflow**

  **Context:** `src/pages/dashboard/Layout.tsx` — tab bar uses `overflowX: auto` but on narrow viewports, the browser-default scrollbar appears (ugly on dark themes).

  **What to do:** Add CSS to hide the scrollbar and use gradient fades:
  ```css
  .dashboard-nav {
    overflow-x: auto;
    scrollbar-width: none; /* Firefox */
    -ms-overflow-style: none; /* IE/Edge */
  }
  .dashboard-nav::-webkit-scrollbar {
    display: none; /* Chrome/Safari */
  }
  ```

  Optionally add gradient fade edges using `::before`/`::after` pseudo-elements with `pointer-events: none`.

  **File:** `src/pages/dashboard/Layout.tsx` (inline styles → convert to CSS class)

### 6.5 ErrorBoundary Per Dashboard Pane

- [ ] **Fix U10: Add per-pane ErrorBoundary to dashboard**

  **Context:** `src/pages/dashboard/CostDashboard.tsx` — if any chart component throws during render, the entire DashboardLayout crashes.

  **What the proper version looks like:** Create a lightweight error boundary:
  ```typescript
  // src/components/PaneBoundary.tsx
  import { Component, type ReactNode } from 'react';

  interface Props { children: ReactNode; label?: string; }
  interface State { error: Error | null; }

  export class PaneBoundary extends Component<Props, State> {
    state: State = { error: null };
    static getDerivedStateFromError(error: Error) { return { error }; }
    render() {
      if (this.state.error) {
        return (
          <div style={{ padding: 20, color: 'var(--text-dim)', fontFamily: 'var(--mono)', fontSize: 11 }}>
            {this.props.label ?? 'Panel'} unavailable
          </div>
        );
      }
      return this.props.children;
    }
  }
  ```

  Wrap each `<Pane>` in CostDashboard with `<PaneBoundary label="Cost Chart">`.

  **Files:** Create `src/components/PaneBoundary.tsx`, modify `src/pages/dashboard/CostDashboard.tsx`

### 6.6 CostDashboard Performance

- [ ] **Fix PERF1/PERF2: Fix useCountUp animation jumps and memoize gate pass rate**

  **Context:**
  - PERF1: `useCountUp` in `src/pages/dashboard/CostDashboard.tsx` captures `from` as the current value at effect start, but `val` is excluded from deps. If target changes mid-animation, the next animation starts from the previous *target*, not the interpolated position, causing visual jumps.
  - PERF2: Gate pass rate calculation runs in the render path, not in `useMemo`.

  **What to do for PERF1:** Track current displayed value in a ref:
  ```typescript
  const displayRef = useRef(0);
  useEffect(() => {
    const from = displayRef.current; // always start from what's displayed
    // ... animation logic
    // On each frame: displayRef.current = interpolatedValue;
  }, [val]);
  ```

  **What to do for PERF2:**
  ```typescript
  const gatePassRate = useMemo(() => {
    // compute rate from gate data
  }, [gateData]);
  ```

  **File:** `src/pages/dashboard/CostDashboard.tsx`

---

## 7. Bench Pages

Issues with the bench comparison and detail pages.

### 7.1 BenchRunDetail Missing ID Guard

- [ ] **Fix B23: Add null check and "not found" state to BenchRunDetail**

  **Context:** `src/pages/BenchRunDetail.tsx:274` — navigating to `/bench/run/` without an ID shows "Loading run undefined...". If the ID doesn't match any demo run, loading persists forever.

  **What to do:**
  ```typescript
  const { id } = useParams();

  if (!id) {
    return <div style={/* ... */}>No run ID specified</div>;
  }

  // After fetch attempt:
  if (loaded && !run) {
    return <div style={/* ... */}>Run {id.slice(0, 8)} not found</div>;
  }
  ```

  **File:** `src/pages/BenchRunDetail.tsx`

### 7.2 BenchRunDetail Token Cost Rates

- [ ] **Fix B24: Update hardcoded token pricing or mark as estimated**

  **Context:** `src/pages/BenchRunDetail.tsx:20-32` — hardcoded rates don't match actual Anthropic pricing. Haiku rate is ~3x wrong.

  **What to do:** Either:
  1. Update rates to current pricing (check Anthropic docs)
  2. Or add "(estimated)" label next to cost breakdown values
  3. Or fetch rates from a config/API endpoint

  **File:** `src/pages/BenchRunDetail.tsx`

### 7.3 BenchCompare Same-Run Guard

- [ ] **Fix B25: Prevent selecting same run for both A and B**

  **Context:** `src/pages/BenchCompare.tsx:80` — both selects show the same run list. Selecting the same run for both produces all-zero deltas.

  **What to do:**
  ```typescript
  const runsForA = runs; // all runs
  const runsForB = runs.filter(r => r.id !== selectedA); // exclude A's selection
  ```

  **File:** `src/pages/BenchCompare.tsx`

### 7.4 BenchCompare Loading State

- [ ] **Fix B26/U6: Add proper error and empty states to BenchCompare**

  **Context:** `src/pages/BenchCompare.tsx` — two issues:
  1. B26: When compare endpoint fails and fallback data lacks `results`, "Loading task comparison data..." shows forever
  2. U6: When `runs.length < 2`, no message explains you need at least two runs

  **What to do:**
  ```typescript
  if (runs.length < 2) {
    return (
      <div style={/* ... */}>
        Complete at least 2 benchmark runs to compare results
      </div>
    );
  }

  // After fetch attempt, if taskMatrix is empty:
  if (loaded && taskMatrix.length === 0 && selectedA && selectedB) {
    return <div>Task-level comparison data not available for these runs</div>;
  }
  ```

  **File:** `src/pages/BenchCompare.tsx`

### 7.5 Bench Navigation Links

- [ ] **Add View/Compare links from bench history table**

  **Context:** `src/pages/Bench.tsx` — the history table shows runs but has no links to `/bench/run/:id` or `/bench/compare`. Users can only reach these pages by manually entering URLs.

  **What to do:** Add a "View" link column in the history table that links to `/bench/run/${run.id}`. Add a "Compare" button near the top of the history tab that links to `/bench/compare`.

  **File:** `src/pages/Bench.tsx`

---

## 8. Configuration & URLs

### 8.1 Service URL Configuration

- [ ] **Fix B3: Replace hardcoded localhost URLs with environment config**

  **Context:** `src/hooks/useChain.ts:67-68` has `const MIRAGE_HOST = 'localhost:8545'` hardcoded. Also hardcoded in 8+ curl commands in `src/lib/scenarios.ts`.

  **What the proper version looks like:**
  ```typescript
  // src/lib/config.ts (may already exist as serve-url.ts — extend it)
  export const SERVE_URL = import.meta.env.VITE_SERVE_URL || 'http://localhost:6677';
  export const MIRAGE_URL = import.meta.env.VITE_MIRAGE_URL || 'http://localhost:8545';
  export const MIRAGE_WS_URL = import.meta.env.VITE_MIRAGE_WS_URL || 'ws://localhost:8545';
  ```

  Then update:
  1. `src/hooks/useChain.ts` — use `MIRAGE_URL` and `MIRAGE_WS_URL`
  2. `src/lib/scenarios.ts` — replace all hardcoded `localhost:8545` in curl command strings with `${MIRAGE_URL}`

  **Files:** `src/lib/serve-url.ts` (or create `src/lib/config.ts`), `src/hooks/useChain.ts`, `src/lib/scenarios.ts`

### 8.2 Gate Pattern Deduplication

- [ ] **Fix arch critique #12: Unify gate detection patterns**

  **Context:** Two separate sets of gate detection regex:
  1. `src/hooks/useTerminalSession.ts` `detectFromOutput()` — patterns for compile, test, clippy
  2. `src/lib/scenarios.ts` `gateFailurePatterns` — different patterns for the same gates

  **What the proper version looks like:**
  ```typescript
  // src/lib/gate-detector.ts
  export const GATE_PATTERNS = {
    compile: {
      pass: [/Compiling.*Finished/, /cargo build.*success/i],
      fail: [/error\[E\d+\]/, /could not compile/],
    },
    test: {
      pass: [/test result: ok/, /\d+ passed/],
      fail: [/test result: FAILED/, /FAILED.*tests/],
    },
    clippy: {
      pass: [/Finished.*clippy/, /0 warnings/],
      fail: [/warning:.*clippy/, /error:.*clippy/],
    },
    diff: {
      pass: [/no changes/i],
      fail: [/files changed/],
    },
  } as const;

  export function detectGates(text: string): { gate: string; status: 'pass' | 'fail' }[] {
    const results: { gate: string; status: 'pass' | 'fail' }[] = [];
    for (const [gate, patterns] of Object.entries(GATE_PATTERNS)) {
      for (const re of patterns.fail) {
        if (re.test(text)) { results.push({ gate, status: 'fail' }); break; }
      }
      for (const re of patterns.pass) {
        if (re.test(text)) { results.push({ gate, status: 'pass' }); break; }
      }
    }
    return results;
  }
  ```

  Both `detectFromOutput` and scenario-level detection import and use this shared module.

  **Files:** Create `src/lib/gate-detector.ts`, modify `src/hooks/useTerminalSession.ts`, `src/lib/scenarios.ts`

---

## 9. Visual & UX Polish

### 9.1 Typography Pass

- [ ] **Fix V7: Enforce minimum 11px font size across the codebase**

  **Context:** Multiple files use `fontSize: 8`, `9`, `10` for canvas labels, stat card subtitles, and table headers. Below accessibility minimum.

  **What to do:** Search and fix:
  ```
  grep -rn "fontSize.*['\"]?\(8\|9\|10\)['\"]?" src/ --include='*.ts' --include='*.tsx' --include='*.css'
  ```

  Rules:
  - Body/table text: minimum 12px
  - Labels and subtitles: minimum 11px
  - Canvas labels: minimum 10px (only in zoomable/interactive contexts)

  **Files:** `src/pages/dashboard/CostDashboard.tsx`, `src/pages/dashboard/KnowledgeGraph.tsx`, `src/components/StatCard.tsx` (if exists), `src/components/GateBar.tsx`, `src/components/PrdPipelinePanel.tsx`, and others found by grep.

### 9.2 Empty States for All Data Views

- [ ] **Fix V3/U15: Add informative empty states to every data-dependent panel**

  **Context:** When the server is live but has no data, pages show zeros/empty voids instead of helpful messages. When fully offline, some pages show demo data but others show nothing.

  **What to do for each page:**

  | Page | Empty state message |
  |------|-------------------|
  | Explorer episodes | "No episodes recorded yet — run `roko plan run` to generate data" |
  | Explorer events | "No events — activity will appear here in real-time" |
  | Explorer providers | "No provider health data available" |
  | Knowledge Graph | "No knowledge shards yet — agents build knowledge as they work" |
  | Knowledge Entries | "No entries — knowledge accumulates as agents complete tasks" |
  | Cascade Routing | "No routing data — the cascade router learns from agent dispatches" |
  | Agent Fleet | "No agents registered — create agents with `roko agent create`" |
  | Dashboard Dreams | "No dream consolidation data — run `roko knowledge dream run`" |

  Style: `fontFamily: 'var(--mono)', fontSize: 12, color: 'var(--text-dim)', textAlign: 'center', padding: '40px'`

  **Files:** Every dashboard page component in `src/pages/dashboard/`, `src/pages/Explorer.tsx`

### 9.3 Explorer Search Fix

- [ ] **Fix U2: Search only meaningful fields in Explorer**

  **Context:** `src/pages/Explorer.tsx` — search uses `JSON.stringify(ep).toLowerCase().includes(s)`. Searching "agent" matches every episode because all have an `agent_id` field.

  **What to do:**
  ```typescript
  const searchableText = [ep.id, ep.agent_id, ep.task_id, ep.output, ep.status]
    .filter(Boolean)
    .join(' ')
    .toLowerCase();
  return searchableText.includes(searchTerm.toLowerCase());
  ```

  **File:** `src/pages/Explorer.tsx`

### 9.4 GateBar Always-Render

- [ ] **Fix P10: Always render GateBar container to prevent layout shifts**

  **Context:** `src/pages/Demo.tsx` — GateBar conditionally renders only when `gates.length > 0`. It pops in/out during demo, causing layout jumps.

  **What to do:** Always render the GateBar container. When empty, show a subtle placeholder:
  ```tsx
  <GateBar gates={gates} />
  // Inside GateBar component:
  if (gates.length === 0) {
    return <div className="gate-bar gate-bar-empty">gates: waiting...</div>;
  }
  ```

  **Files:** `src/pages/Demo.tsx`, `src/components/GateBar.tsx`

### 9.5 Demo Progressive Disclosure

- [ ] **Redesign Demo page to use phase-driven progressive disclosure**

  **Context:** Currently the Demo page shows everything at once: pipeline, constellation, tasks, terminal output, stats, timeline, gates — all visible simultaneously. Too many competing focal points.

  **What the proper version looks like** (from `09-REDESIGN-PROPOSALS.md`):

  **Before play (idle):** Clean screen with scenario description, play button, and phase rail (○ idea ○ PRD ○ plan ○ tasks ○ execute ○ verify). No terminals, no stats, no gates visible.

  **During execution:** Only relevant panels visible for current phase. Terminal is subordinate (20-30% of space). The current phase's artifact is the focal point.

  **Implementation approach:**
  1. Add a `phase` state to Demo.tsx that tracks which pipeline phase is active
  2. Conditionally render sections based on phase:
     - `idle`: description + play button + phase rail
     - `idea`/`prd`: artifact panel (left, 60%) + terminal (right, 40%)
     - `planning`: task list (dominant) + terminal (compact)
     - `executing`: task board with per-task status + terminal (compact) + gates (bottom)
     - `complete`: summary with results
  3. Phase transitions use CSS `opacity`/`transform` animations (fade-up using existing `.reveal` class)

  This is a significant refactor of `src/pages/Demo.tsx`. The layout logic needs to change from a fixed grid to a phase-dependent composition.

  **Files:** `src/pages/Demo.tsx`, `src/pages/Demo.css` (if exists)

---

## 10. Dead Code & Cleanup

### 10.1 Screenshot Script Fix

- [ ] **Fix DC6: Remove `/jobs` route from screenshot script**

  **Context:** `screenshot-all.mjs` references a `/jobs` route that doesn't exist. Produces a 404 screenshot.

  **What to do:** Remove the `/jobs` entry from the routes array in the script.

  **File:** `screenshot-all.mjs` (in demo-app root or scripts directory)

### 10.2 Pipeline State Spreading Fix

- [ ] **Fix arch critique #10: Make `stream` required on PipelineDemoState**

  **Context:** `src/pages/Demo.tsx` — `patchPipelineStream` uses `...(prev.stream ?? {})` because `stream` is optional. The `?? {}` guard was added to fix a runtime crash.

  **What to do:** Make `stream` required in the `PipelineDemoState` type definition. Ensure all factory functions (`createPipelineIntroState()`, etc.) include `stream` in their return value. Remove the `?? {}` guard.

  **File:** `src/pages/Demo.tsx` (type definition and factory functions)

### 10.3 Dead Playback Mode Code

- [ ] **Fix arch critique #2/#9: Remove auto-mode code from PlaybackController**

  **Context:** `src/lib/playback.ts` — the Auto/Step toggle UI was removed but the auto-mode code path remains. `setPlaybackMode` is suppressed with `void`. `_progressText` and `_progressLabel` are set but never rendered.

  **What to do:** If implementing the DemoController (item 1.1), this is absorbed. If not, simplify `PlaybackController`:
  1. Remove `setMode()` / `getMode()` — always step mode
  2. Remove `_progressText`, `_progressLabel` and their callback registrations
  3. Remove `void setPlaybackMode` suppression

  **File:** `src/lib/playback.ts`

---

## 11. Performance

### 11.1 AgentFleet Draw Callback Stability

- [ ] **Fix PERF3: Only restart simulation when actual topology changes**

  **Context:** `src/pages/dashboard/AgentFleet.tsx` — `draw` is `useCallback([data])`. Every poll that detects a topology change (even ordering differences per B38) recreates `draw`, which restarts the entire physics simulation from scratch.

  **What to do:** Memoize the topology data by its structural content (node/edge IDs and statuses), not by object identity. Only recreate `draw` when the structural content actually changes:
  ```typescript
  const topologyKey = useMemo(() => {
    if (!data) return '';
    const nodes = [...data.nodes].sort((a, b) => a.id.localeCompare(b.id));
    return nodes.map(n => `${n.id}:${n.status}`).join(',');
  }, [data]);

  const draw = useCallback(() => {
    // ... drawing logic
  }, [topologyKey]); // stable across identical topologies
  ```

  **File:** `src/pages/dashboard/AgentFleet.tsx`

---

## Summary

| Section | Items | Key Impact |
|---------|-------|-----------|
| 1. Demo Playback Engine | 6 | Speed/pause/step actually work across all 15 scenarios |
| 2. Terminal System | 6 | No duplicate keystrokes, no timer leaks, no rainbow bar |
| 3. Data Layer | 7 | Server recovery detected, errors distinguished from fallbacks |
| 4. Demo Page Architecture | 4 | Clean unmount, no stale callbacks, no dead code |
| 5. Scenario Bugs | 3 | No crashes on null entries, no leaked intervals |
| 6. Dashboard Pages | 6 | Pipeline failure display, parallel polling, error boundaries |
| 7. Bench Pages | 5 | Proper loading/error/empty states, working navigation |
| 8. Configuration & URLs | 2 | Deployable beyond localhost, unified gate patterns |
| 9. Visual & UX Polish | 5 | Readable text, informative empty states, progressive disclosure |
| 10. Dead Code & Cleanup | 3 | Cleaner codebase, smaller bundle |
| 11. Performance | 1 | No jarring simulation restarts |
| **Total** | **48** | |

### Recommended execution order

**Phase 1 — Make the demo work correctly** (items 1.1–1.6, 2.1, 5.1):
The playback engine + terminal listener fix. After this, speed/pause/step/reset all work. No duplicate keystrokes.

**Phase 2 — Data resilience** (items 3.1–3.4, 3.6, 3.7):
Server probe re-probes, errors are discriminated, data mode tracks correctly.

**Phase 3 — Visual quality** (items 9.1–9.5, 6.4, 6.5):
Typography pass, empty states everywhere, progressive disclosure on Demo page.

**Phase 4 — Bench & cleanup** (items 7.1–7.5, 10.1–10.3, 4.3–4.4):
Bench pages work end-to-end, dead code removed.

**Phase 5 — Polish** (items 8.1–8.2, 11.1, remaining):
URL config, gate deduplication, performance tuning.
