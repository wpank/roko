# 128 — Adaptive Frame Rate (Drop to 20 fps When Idle)

**Priority**: P3 — Rendering at 60 fps continuously wastes CPU when agents are running and no user input is being received; dropping to 20 fps during idle periods reduces CPU consumption without affecting responsiveness.
**Size**: XS (2-3 hours)
**Crates**: `crates/roko-cli/src/tui/mod.rs`, `crates/roko-cli/src/tui/app.rs`
**Depends on**: None
**Sources**: `tmp/backlog/_checklist-gaps.md` §2.7

---

## Background

The roko TUI renders at a fixed tick rate (likely 60 fps based on the 16ms tick interval common in ratatui applications). When agents are executing and no user is actively watching the TUI, this continuous rendering consumes CPU on the developer's machine that could otherwise go to the build processes and LLM response parsing.

Mori solved this by tracking the last user input timestamp and dropping the render rate from 60 fps to 20 fps after 5 seconds of inactivity. The rate reverts to 60 fps immediately on any key or mouse event. This is a three-line change: compare the elapsed time since last input to a threshold, and choose the tick interval accordingly.

The TUI still needs to update on data changes (new runner events), so the idle rate is 20 fps rather than stopped. This ensures events are rendered within 50ms of arrival even during idle.

## Current State

- `crates/roko-cli/src/tui/mod.rs` — TUI event loop with a fixed tick interval.
- Last input timestamp: not tracked.
- No adaptive tick mechanism exists.

## Implementation Plan

1. **Track last input time**: In `app.rs` or the TUI event loop, add `last_input_at: Instant` initialized to `Instant::now()`. Update it on every key event and mouse event.

2. **Adaptive tick interval**: At the start of each iteration of the TUI event loop:
   ```rust
   let tick_interval = if last_input_at.elapsed() > Duration::from_secs(5) {
       Duration::from_millis(50)   // 20 fps
   } else {
       Duration::from_millis(16)   // 60 fps
   };
   ```

3. **Apply to `crossterm::event::poll`**: Pass `tick_interval` to the event poll timeout (or to a `tokio::time::sleep` if using async rendering).

4. **Resume immediately on input**: The `last_input_at = Instant::now()` update on each key event ensures the next tick uses the 16ms interval, providing instant responsiveness.

5. **Optional config**: Add `[tui] max_fps = 60` and `[tui] idle_fps = 20` to `roko.toml` schema with the above defaults.

## Acceptance Criteria

1. After 5 seconds of no user input, CPU usage from the TUI process drops measurably.
2. Pressing any key immediately resumes 60 fps rendering (verified by smooth animation).
3. Runner events (plan state changes) still appear within 50ms during idle.
4. No visible rendering artifacts from switching between tick rates.

## Verification Checklist

- [ ] Run `top` or `htop` while roko dashboard is in idle mode; verify CPU is lower than during active use.
- [ ] Press a key after 10 seconds of idle; verify the TUI responds immediately with no visual lag.
- [ ] Start a plan; verify the TUI updates promptly at 20 fps (within 50ms per event).

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/tui/mod.rs` | Add adaptive tick interval based on `last_input_at` |
| `crates/roko-cli/src/tui/app.rs` | Add `last_input_at: Instant`; update on key/mouse events |
