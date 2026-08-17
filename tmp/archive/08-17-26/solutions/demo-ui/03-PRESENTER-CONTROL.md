# Presenter Control Issues

Everything that affects the demo flow, pacing, and presenter experience during a live presentation.

---

## Blockers (Demo Breaks)

### P1. Speed button does nothing

**Experience:** The speed button cycles through `0.5x / 1x / 2x / 4x` and the label updates visually. The demo runs at exactly the same pace regardless.

**Root cause:** `speedIdx` state lives in `Demo.tsx` but is never passed to `PlaybackController`, `TimelineStepper`, or any scenario function. `setGlobalSpeed()` was wired to `useTerminalSession` (typing speed only), but `rawSleep()` delays in scenarios are hardcoded millisecond values that ignore the speed setting entirely.

**Affected:** All scenarios.

**Fix:** Create a speed-aware sleep: `await demoSleep(ms, globalSpeed)` that divides the delay by the speed multiplier. Replace all `rawSleep()` calls in scenarios with `demoSleep()`.

---

### P2. `rawSleep()` is opaque to pause, step, and stop

**Experience:** Pressing Pause or Next has no effect while the scenario is inside a `rawSleep()` call. The scenario sits for the full duration. Pressing Reset works eventually but only because `runningRef.current = false` fires synchronously — the sleeping code wakes up after timeout and continues for one more iteration.

**Root cause:** `rawSleep` is a bare `setTimeout`:
```ts
function rawSleep(ms: number): Promise<void> {
  return new Promise(r => setTimeout(r, ms));
}
```
No awareness of pause, step, or running state.

**Affected:** Nearly every scenario. Worst offenders:
- `chat` — uses `rawSleep(300)`, `rawSleep(800)`, `rawSleep(500)`, `rawSleep(3000)`, `rawSleep(2000)` inline
- `useTerminalSession.ts` — `rawSleep(200)`, `rawSleep(100)` in `waitForOpen`
- `useTerminal.ts` — `sleep()` in `waitForPrompt`, `waitForMarker`, `typeCmd`

**Fix:** Replace with an interruptible sleep that polls `running.current` and `paused.current` every 50ms:
```ts
async function demoSleep(ms: number, ctx: { running: RefObject<boolean>, paused: RefObject<boolean>, speed: number }) {
  const end = Date.now() + ms / ctx.speed;
  while (Date.now() < end) {
    if (!ctx.running.current) return;
    while (ctx.paused.current) await new Promise(r => setTimeout(r, 50));
    await new Promise(r => setTimeout(r, Math.min(50, end - Date.now())));
  }
}
```

---

### P3. Pause has no effect during running `showCmd`

**Experience:** Pressing Pause during a long `showCmd` (e.g., `roko plan run` with 420s timeout) does nothing. The command continues executing. Pause only takes effect at `waitForStep()` boundaries.

**Root cause:** `pausedRef.current` is only checked in explicit `while (paused.current) await rawSleep(100)` loops that a few scenarios have between steps. The core `showCmd` → `typeVisibleCommandAndWait` → `waitForMarker` chain has no pause awareness.

**Scenarios that correctly poll pause:**
- `gateRetry` (via `advance()` helper)
- `dreamConsolidation` (via `allowPhaseTime()`)
- `chainIntelligence` (inline guards)
- `explore` (inline guards)

**Scenarios that completely ignore pause during execution:** `prdPipeline`, `selfhost`, `prdResearchLoop`, `knowledgeAccumulation`, `knowledgeTransfer`, `race`, `providers`, `providerRace`, `chat`.

**Fix:** Thread the `paused` ref through `showCmd`. The terminal output continues streaming but the scenario's progress tracking pauses until unpaused.

---

### P4. `race`, `providers`, `explore` completely bypass playback controls

**Experience:** In these 3 scenarios, pressing Pause, Next, or changing mode does nothing throughout the entire run. All work is in `Promise.all` with no `waitForStep()` calls.

- `race`: Two `showCmd` in `Promise.all`. No step gates.
- `providers`: Four `showCmd` in `Promise.all`. No step gates.
- `explore`: Four parallel panes, each running 3 commands. Has `paused.current` guards but no `waitForStep()`.

**Fix:** Add `waitForStep()` before the parallel launch and after completion. For `explore`, add step gates between the 3 sequential commands in each pane.

---

## Friction (Demo Feels Janky)

### P5. Reset doesn't kill server-side PTY processes

**Experience:** Pressing Reset stops the scenario from advancing but the PTY on the server continues. Terminal output keeps streaming for seconds/minutes after Reset.

**Root cause:** `handleReset` regenerates `sessionIds` causing terminal components to remount, which closes the client-side WebSocket. But the server's PTY process keeps running with no kill signal.

**Fix:** Send a kill signal to the server before closing the WS. Or accept the behavior — the new session gets fresh terminals.

---

### P6. Stale context callbacks bleed into new scenario after tab switch

**Experience:** Clicking a different scenario tab while a scenario is running sets `runningRef.current = false`, but the old scenario's promises are still resolving. When they do, they call `ctx.setGate`, `ctx.logCommand`, `ctx.setMetric` — which update the new scenario's sidebar with stale data.

**Root cause:** The old `ctx` object's callbacks are closures over the current component's `setState` setters. They are never nulled out.

**Fix:** Guard all ctx callbacks with a `generationRef` counter. Increment it on scenario switch. Each callback checks if the generation matches before calling setState.

---

### P7. `prdResearchLoop` phase 6 (gate results) has no `waitForStep()`

**Experience:** After `plan run` finishes, the scenario immediately processes gate results and jumps to phase 7 without pausing. The presenter cannot talk about gate results.

**Fix:** Add `await playback.waitForStep()` before phase 6 processing.

---

### P8. `knowledgeTransfer` starts immediately with no intro step

**Experience:** Pressing Play immediately starts setting up two workspaces. The presenter has no opportunity to introduce the scenario.

**Fix:** Add `await playback.waitForStep()` before the first `setupWorkspace` call.

---

### P9. `providerRace` tracker leak if presenter never advances

**Experience:** After the race finishes, `trackers` keep polling every 250ms. If the presenter pauses to discuss results but never presses Next, trackers accumulate `setMetric` calls indefinitely.

**Fix:** Clear trackers when the race promise resolves, not only in the `finally` block.

---

### P10. GateBar pops in/out causing layout jumps

**Experience:** The gate bar conditionally renders only when `gates.length > 0`. During a demo, it pops in when gates are first detected and disappears on reset, causing a visible layout shift.

**Fix:** Always render the GateBar container. Show "waiting..." or an empty bar when no gates exist.

---

### P11. `handlePauseResume` stale closure risk

**Experience:** Clicking Pause then Resume rapidly can get the `pausedRef` out of sync with the `isPaused` state.

**Root cause:** `isPaused` in the closure is the value at last `useCallback` recomputation. Two rapid clicks during a pending render can disagree.

**Fix:** Use a functional setState updater: `setIsPaused(prev => { pausedRef.current = !prev; return !prev; })`.

---

## Cosmetic

### P12. `typeCmd` ignores speed setting

**Experience:** The `chat` scenario types commands at fixed 12ms + random 6ms per character. Speed button has no effect.

**Fix:** Multiply character delay by `1 / globalSpeed`.

---

### P13. `mirage` scenario has empty `steps: []`

**Experience:** The timeline sidebar is empty for this scenario.

**Fix:** Either add timeline steps or mark this scenario as `panel: false` (it already is, so the sidebar isn't shown — this is cosmetic only).

---

## Step Mode Coverage Matrix

Which scenarios support which playback controls:

| Scenario | `waitForStep()` calls | Pause guards | Speed-aware | Step mode works? |
|----------|----------------------|-------------|-------------|-----------------|
| prdPipeline | 5 | No | No | Partial (between phases only) |
| selfhost | 5 | No | No | Partial |
| prdResearchLoop | 7 (missing phase 6) | No | No | Partial |
| race | 0 | No | No | **No** |
| gateRetry | 6 (via `advance()`) | Yes | No | **Yes** |
| providers | 0 | No | No | **No** |
| providerRace | 2 | No | No | Partial |
| explore | 0 | Yes | No | **No** |
| knowledgeAccumulation | 6 | No | No | Partial |
| dreamConsolidation | 7 | Yes | No | **Yes** |
| chat | 3 | No | No | Partial |
| knowledgeTransfer | 4 (missing intro) | No | No | Partial |
| chainIntelligence | 6 | Yes | No | **Yes** |
| mirage | 0 (instant) | N/A | N/A | N/A |
| builder | 0 (external) | N/A | N/A | N/A |

**Only 3 of 15 scenarios fully support step mode.** None are speed-aware.

---

## Priority Fix Order

1. **P1 + P2**: Wire speed into all timing functions. This is the single highest-impact fix.
2. **P4**: Add `waitForStep()` to race/providers/explore. Step mode must work everywhere.
3. **P3**: Thread pause through `showCmd`. Pause must actually pause.
4. **P6**: Guard stale callbacks on scenario switch. Prevents bleed.
5. **P7 + P8**: Add missing `waitForStep()` calls. Quick fixes.
6. **P10**: Always render GateBar container. Quick CSS fix.
7. **P5**: Server-side PTY kill on reset. Requires backend work.
