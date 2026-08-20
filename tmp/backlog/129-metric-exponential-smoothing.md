# 129 — Metric Exponential Smoothing (Prevent Visual Jumps)

**Priority**: P3 — Metrics that jump from 0 to large values (token counts, CPU%, progress %) create visual noise that makes it harder to track trends; exponential smoothing gives a stable, readable display.
**Size**: XS (2-3 hours)
**Crates**: `crates/roko-cli/src/tui/app.rs`, `crates/roko-cli/src/tui/tabs.rs`
**Depends on**: None
**Sources**: `tmp/backlog/_checklist-gaps.md` §2.8

---

## Background

When a task completes and the token count jumps from 0 to 12,400 tokens in a single frame, or when a build finishes and CPU% drops from 90% to 5% instantly, the visual update is jarring. Operators tracking trends by watching the numbers can lose context on where the value was before the jump.

Exponential smoothing (EMA) is the standard fix: the displayed value is a weighted average of the previous displayed value and the new raw value, with a smoothing factor `alpha` that controls how quickly the display tracks the real value. With `alpha = 0.12`, each frame the displayed value moves 12% of the way from the current display to the raw value. This produces smooth transitions without adding lag at the higher end (large jumps still move quickly; small noise is damped).

Implementation is a simple wrapper type or helper function applied at the render sites where numeric values are displayed.

## Current State

- Metric rendering in `tabs.rs` and `app.rs` displays raw values from `TuiModel` directly.
- No smoothing is applied to any numeric display.
- Token counts, CPU%, memory, and progress fractions all update discretely.

## Implementation Plan

1. **`SmoothedValue` wrapper type** in a new `crates/roko-cli/src/tui/smoothing.rs`:
   ```rust
   pub struct SmoothedValue {
       alpha: f64,         // smoothing factor, default 0.12
       display: f64,       // current smoothed display value
   }

   impl SmoothedValue {
       pub fn new(alpha: f64) -> Self { ... }
       pub fn update(&mut self, raw: f64) { self.display = self.alpha * raw + (1.0 - self.alpha) * self.display; }
       pub fn get(&self) -> f64 { self.display }
   }
   ```

2. **Apply to `TuiModel` metrics**: For each metric that should be smoothed, store a `SmoothedValue` in `TuiModel` alongside the raw value:
   - `plans_completed_fraction: SmoothedValue` (for progress bar)
   - `cpu_pct: SmoothedValue` (for header system metrics)
   - `memory_bytes: SmoothedValue` (for header)
   - `token_count_per_role: HashMap<String, SmoothedValue>` (for F7 prompt stats)

3. **Update on each frame**: In the TUI render loop, call `smoothed_value.update(raw_value)` for each metric before rendering. The raw value comes from the latest `DashboardSnapshot`; the smoothed display value is what gets rendered.

4. **Use smoothed values in rendering**: In `tabs.rs`, replace `model.plans_completed as f64 / model.plans_total as f64` in the progress bar with `model.plans_completed_fraction.get()`.

5. **Skip smoothing for discrete state**: Status fields (`running`, `completed`, `failed`) and counters that must be exact (episode count, playbook rule count) should NOT be smoothed. Only continuous numeric metrics benefit from smoothing.

## Acceptance Criteria

1. Progress bar fills smoothly when tasks complete (no instantaneous jump).
2. CPU% in the header updates gradually rather than jumping.
3. Token counts per role in F7 smooth out rather than jumping on task completion.
4. Discrete state fields (plan status, error count) are NOT smoothed.
5. After 10 frames, the smoothed value is within 10% of the raw value for `alpha = 0.12`.

## Verification Checklist

- [ ] Watch the progress bar during a task completion; verify it fills smoothly over multiple frames.
- [ ] Check that `SmoothedValue::update` converges: after 50 frames with raw=100 and start=0, `get()` ≥ 99.
- [ ] Run unit test: `SmoothedValue::new(0.12)` → 10 updates with raw=100 → `get()` is approximately 72.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/tui/smoothing.rs` | New file: `SmoothedValue` type |
| `crates/roko-cli/src/tui/mod.rs` | Export `smoothing` module |
| `crates/roko-cli/src/tui/app.rs` | Add `SmoothedValue` fields to `TuiModel` |
| `crates/roko-cli/src/tui/tabs.rs` | Use smoothed values at render sites |
