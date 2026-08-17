# Architecture Critique: Bandaids vs Proper Design

## 1. Speed Control — Bandaid

**What exists:** A `globalSpeed` module-level variable in `useTerminalSession.ts` that `showCmd` reads as a default. A `speedRef` in Demo.tsx. A `ctx.speed` on ScenarioContext.

**The bandaid:** Three separate speed representations that must be manually kept in sync:
- `globalSpeed` (module singleton)
- `speedRef.current` (React ref)
- `ctx.speed.current` (scenario context)
- `SPEEDS[speedIdx]` (React state for display)

The `cycleSpeed` callback updates all three, but scenarios never read `ctx.speed.current` for their `rawSleep()` delays. Speed only affects typing animation, not scenario pacing.

**Proper design:** A single `DemoController` class that owns speed, pause state, and step gating. Scenarios receive this controller and all timing-sensitive operations go through it:
```typescript
class DemoController {
  speed: number;
  paused: boolean;
  async sleep(ms: number): Promise<void> { /* respects speed + pause */ }
  async waitForStep(): Promise<void> { /* blocks until presenter advances */ }
  async showCmd(handle, cmd, opts): Promise<CommandResult> { /* uses speed */ }
}
```
This eliminates the ref/global/context triple, and makes every scenario automatically respect speed and pause without individual wiring.

---

## 2. Playback Mode — Bandaid

**What exists:** `PlaybackController` with `setMode('auto'|'step')`, `waitForStep()`, `advanceStep()`. Default forced to 'step' in a useEffect. The Auto/Step toggle UI was removed but the code for it remains.

**The bandaid:** The mode toggle was removed from the UI but the entire auto-mode code path still exists in `PlaybackController`. The `setPlaybackMode` setter is retained with `void setPlaybackMode` to suppress lint. `_progressText` and `_progressLabel` state are set by callbacks but never rendered anywhere.

**Proper design:** Remove auto mode entirely from `PlaybackController`. It should only support step mode for presenter control. Remove `_progressText`, `_progressLabel`, `setPlaybackMode`. The controller becomes simpler:
```typescript
class PlaybackController {
  private stepResolve: (() => void) | null = null;
  waitForStep(): Promise<void> { /* always blocks */ }
  advanceStep(): void { /* resolves the promise */ }
  reset(): void { /* clears pending */ }
}
```

---

## 3. Terminal Handle Management — Bandaid

**What exists:** `handleRefs` is a `useRef<RefObject<TerminalHandle | null>[]>([])` that gets rebuilt on scenario change. Each `TerminalPaneWithHandle` writes its handle into the ref via a bare useEffect (no deps).

**The bandaid:** The ref-of-refs pattern is fragile. The bare useEffect in `TerminalPaneWithHandle` runs on every render, which means the handle ref is continuously overwritten even when nothing changed. The `buildContext()` filters nulls from the refs but doesn't guard against stale handles from previous scenarios.

**Proper design:** A `TerminalManager` that owns handle lifecycle:
```typescript
class TerminalManager {
  private handles = new Map<string, TerminalHandle>();
  register(id: string, handle: TerminalHandle): void;
  unregister(id: string): void;
  get(index: number): TerminalHandle | null;
  getAll(): TerminalHandle[];
}
```
Pass this to scenarios instead of raw entries array. Handles register themselves on mount and unregister on unmount.

---

## 4. WebSocket Listener Accumulation — Bandaid

**What exists:** `useTerminal.ts` creates `term.onData()` and `term.onResize()` listeners inside `connectWs()`, which is called on initial mount AND on every reconnection (every 2 seconds when disconnected).

**The bandaid:** There is no cleanup of previous listeners when reconnecting. After N reconnections, there are N copies of each listener. Terminal input gets duplicated N times.

**Proper design:** The xterm event listeners should be registered ONCE when the terminal is created, not inside `connectWs()`. The WS connection management should be separate from terminal setup:
```typescript
// Terminal setup (once):
const term = new Terminal(opts);
term.onData(data => wsRef.current?.send(data));
term.onResize(size => wsRef.current?.send(resize(size)));

// WS lifecycle (can reconnect freely):
function connectWs() {
  const ws = new WebSocket(url);
  ws.onmessage = (e) => term.write(e.data);
  wsRef.current = ws;
  // onData/onResize already reference wsRef, so reconnection just works
}
```

---

## 5. Scenario Error Handling — Missing

**What exists:** Only `prdPipeline` scenario checks command results. 12+ other scenarios fire `showCmd` and ignore whether it succeeded.

**The bandaid:** None — this is simply missing. Errors are swallowed silently.

**Proper design:** Each scenario should have an error boundary pattern:
```typescript
async function runWithErrorHandling(
  ctx: ScenarioContext,
  steps: () => AsyncGenerator<void>,
): Promise<void> {
  try {
    for await (const _ of steps()) { /* step completed */ }
  } catch (err) {
    ctx.logCommand('ERROR', err instanceof Error ? err.message : 'Unknown error');
    ctx.setGate('scenario', 'fail');
  } finally {
    // Clean up any active intervals/trackers
  }
}
```
Or simpler: a `showCmdOrFail` helper that throws on failure, wrapped in a top-level try/catch per scenario.

---

## 6. Interval/Tracker Cleanup — Bandaid

**What exists:** `trackMetrics()` returns a `setInterval` ID. Scenarios manually call `clearInterval(tracker)` in specific success-path locations.

**The bandaid:** If the scenario throws, is interrupted by the presenter pressing Reset, or if the terminal disconnects, the interval keeps running indefinitely. Some scenarios (gateRetry, knowledgeTransfer, chainIntelligence) have 2-3 active trackers that can all leak.

**Proper design:** Use an `AbortController` pattern or a cleanup registry:
```typescript
class ScenarioCleanup {
  private cleanups: (() => void)[] = [];
  track(interval: ReturnType<typeof setInterval>): void {
    this.cleanups.push(() => clearInterval(interval));
  }
  dispose(): void {
    this.cleanups.forEach(fn => fn());
    this.cleanups = [];
  }
}
```
Scenarios receive this and register all intervals. `handleReset` calls `cleanup.dispose()`. The `finally` block in `handlePlay` also calls it.

---

## 7. Scenario Context — Overloaded

**What exists:** `ScenarioContext` is a 16-field interface carrying terminal handles, playback controller, timeline stepper, metric setters, gate setters, logger, pipeline state setters (5 of them), pipeline example, pause/running/speed refs.

**The bandaid:** Every scenario receives all 16 fields even if it uses 3. The `chat` scenario doesn't use pipeline state. The `builder` scenario doesn't use gates. The `mirage` scenario doesn't use timeline. But all pay the cognitive cost of a massive destructured parameter list.

**Proper design:** Compose from smaller interfaces:
```typescript
interface BaseContext {
  entries: TerminalHandle[];
  controller: DemoController; // speed + pause + step
  timeline: TimelineStepper;
  log: (cmd: string, desc: string) => void;
}

interface MetricContext {
  setMetric: (id: string, value: string) => void;
  setGate: (name: string, status: 'pass' | 'fail' | 'pending') => void;
}

interface PipelineContext {
  setPipeline: (state: PipelineDemoState) => void;
  patchPipeline: (patch: Partial<PipelineDemoState>) => void;
  // ... etc
}
```
Scenarios declare which contexts they need. Simpler scenarios get simpler types.

---

## 8. Route Duplication — Bug

**What exists:** `main.tsx` has two routes for `/share/:token` — one inside the dashboard layout rendering `ShareView`, another at the top level rendering `SharePage`.

**The bandaid:** The second route is unreachable. React Router matches the first one.

**Proper design:** Decide which component should handle `/share/:token` and remove the other. If both views are needed, use different paths (e.g., `/share/:token` vs `/receipt/:token`).

---

## 9. Progress State — Dead Code

**What exists:** `_progressText` and `_progressLabel` state in Demo.tsx. Updated by `playback.onProgress()` callback but never rendered.

**The bandaid:** The underscore prefix and eslint-disable comments acknowledge this is dead code but keep it "in case we need it later."

**Proper design:** Delete it. If needed later, it can be re-added. Dead code with lint suppressions is noise.

---

## 10. Pipeline Stream Spreading — Fragile

**What exists:** `patchPipelineStream` in Demo.tsx uses `...(prev.stream ?? {})` because `stream` is optional on `PipelineDemoState`.

**The bandaid:** The `?? {}` was added to fix a runtime crash where `prev.stream` was undefined. But the root cause is that `createPipelineIntroState()` didn't include `stream` in its return value.

**Proper design:** Make `stream` required on `PipelineDemoState` with a default value. The `EMPTY_PIPELINE_STATE` already has it. Ensure all factory functions include it. Remove the `??` guard since it can never be undefined.

---

## 11. Hardcoded Service URLs — Missing Configuration

**What exists:** `localhost:8545` hardcoded in useChain.ts, useJobEvents.ts, and 8 curl commands baked into scenario strings in scenarios.ts.

**The bandaid:** Works on localhost but breaks in any other deployment environment.

**Proper design:** A single `config.ts` that derives service URLs from environment:
```typescript
export const SERVE_URL = import.meta.env.VITE_SERVE_URL || 'http://localhost:6677';
export const MIRAGE_URL = import.meta.env.VITE_MIRAGE_URL || 'http://localhost:8545';
```
Scenarios should use `${MIRAGE_URL}` in their command strings.

---

## 12. Gate Pattern Matching — Brittle

**What exists:** Two sets of gate detection regex patterns — one in `useTerminalSession.ts:detectFromOutput()` and another in `scenarios.ts:gateFailurePatterns`. They use different patterns and don't share code.

**The bandaid:** Duplicate detection logic with slightly different regex patterns for the same concepts.

**Proper design:** A single `gate-detector.ts` module with a shared pattern set:
```typescript
export const GATE_PATTERNS = {
  compile: { pass: /.../, fail: /.../ },
  test: { pass: /.../, fail: /.../ },
  // etc
};
export function detectGates(text: string): GateResult[] { ... }
```
Both `detectFromOutput` and scenario-level detection use this shared module.
