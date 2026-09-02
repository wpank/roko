# 09 -- Animation, Transitions, and Visual Effects Audit

**Date:** 2026-09-01
**Scope:** Everything that moves, fades, pulses, transitions, or otherwise changes over time in the TUI -- measured against the Mori/Bardo spec.

---

## 1. Current Animations Inventory

### What actually animates today

| Animation | Where | Mechanism | Quality |
|---|---|---|---|
| **Braille spinner** | `atmosphere.rs:87` | Frame-counter cycling through `['⠋','⠙','⠹','⠸','⠼','⠴','⠦','⠧','⠇','⠏']` every 4 frames | Good. Smooth, visually distinctive. |
| **Ethereal spinner** | `atmosphere.rs:94` | Slower variant `['◜','◝','◞','◟']` every 8 frames | Good. Subtle alternative. |
| **Heartbeat pulse** | `atmosphere.rs:40` | Double-beat pattern: two quick rise/fall cycles in 1.5s, then rest | Good. Biologically convincing two-pulse systole. |
| **Breathing brightness** | `atmosphere.rs:33` | Sine wave `0.9 + 0.1 * sin(t * PI * 0.5)`, range 0.8..1.0 | Good. Slow, subliminal. |
| **Sparkline updates** | `widgets/braille.rs` | Data push updates braille dot patterns each refresh | No animation -- instant replacement. Data jumps. |
| **Task progress bar heartbeat** | `widgets/task_progress.rs:257` | Leading edge brightness pulsed by `atmosphere.heartbeat()` | Minimal. Clamped to 0.9..1.1 scale so nearly invisible. |
| **Wave progress gradient** | `widgets/wave_progress.rs:82` | Per-cell ocean gradient with `(j/width + elapsed * 0.1) % 1.0` offset | Good. Gives the filled bar a slow flowing-water feel. |
| **Active task icon pulse** | `widgets/task_progress.rs:136` | `pulse_rose(heartbeat)` modulates RGB of the `►` indicator | Subtle. ~10% brightness variation. |
| **Particle overlay** | `postfx.rs:530` | Floating braille dots with Lissajous drift, lifetime decay, 3x3 clearance | Well-built. Respects content, fades over 5.5s lifetime. |
| **Data rain** | `postfx.rs:399` | Matrix-style falling braille streams, speed tied to token throughput | Well-built. Streams fade along trail length. |
| **Activity ripples** | `postfx.rs:333` | Concentric rings expanding from deterministic centers, thickness from activity level | Well-built. Rings write only into blank cells. |
| **Progress field** | `postfx.rs:285` | Bottom-up fill with pulsing braille edge, rose/violet gradient | Well-built. Clear visual progress metaphor. |
| **Guide lines** | `postfx.rs:222` | Pulsing sine-wave traces across blank regions | Well-built. Sine modulation prevents straight-line monotony. |
| **State viz** | `postfx.rs:458` | Background-only slow waves driven by plan progress, activity, token rate, errors | Well-built. Never touches fg or symbols -- pure bg tint. |
| **Self-glow** | `postfx_pipeline.rs:150` | Brightens cells above luminance threshold by fractional headroom | Lightweight. Applied to tabs 0/1/2. |
| **Ambient orbs** | `postfx.rs:845` | Breathing brightness orbs drifting on Lissajous paths, 3x3 influence | Only when `vfx_enabled` (currently never from presets). |
| **Dream atmosphere** | `postfx.rs:892` | Vignette + breathing brightness + film grain | Only when `vfx_enabled`. |
| **Modal glow** | `postfx.rs:780` | Radial falloff tint around modal edges, 6-cell range | Active when `screen_postfx` enabled. Nice halo effect. |
| **Notification TTL** | `modals/notification.rs:58` | `is_expired()` after N seconds, then pruned | No fade -- instant pop-in, instant disappear. |
| **Header heartbeat dot** | `widgets/header_bar.rs:18` | Alternates `●`/`○` characters based on frame count | Binary toggle, not smooth. |
| **Smoothed values** | `smoothing.rs` | EMA with alpha=0.12 for display metrics | Prevents jumps but is a data filter, not a visual animation. |

### Summary count

- **Animating continuously:** 7 (spinner, ethereal spinner, heartbeat, breathing, wave gradient, particles, data rain)
- **Animating on state change:** 3 (progress field, activity ripples, guide lines)
- **Post-processing effects:** 5 (self-glow, bloom, vignette, modal glow, state viz)
- **Latent/disabled:** 2 (ambient orbs, dream atmosphere -- behind `vfx_enabled` which no preset enables)
- **Total moving elements:** ~17

---

## 2. Transitions Between Views/Tabs

### Current state: hard cut

Tab switching is implemented in `tabs.rs` as `Tab::next()` / `Tab::prev()` -- a pure enum swap. The `app.rs` draw loop picks up the new `tui_state.active_tab` on the next frame and renders the target view. There is:

- **No transition state** tracked between old and new tab
- **No previous-frame buffer** retained for crossfade/blend
- **No animation timeline** for the switch
- **No directional awareness** (going left vs. right)
- **No scan line, sweep, dissolve, or fade** of any kind

The user presses F3, the screen instantly becomes the Agents view. No temporal connective tissue.

### What the spec demands (03-transitions.md)

The spec defines a five-tier transition system:

| Tier | Duration | Examples |
|---|---|---|
| **T0: Ambient Pulse** | 50-200ms | Border brightens on focus, value flash on update |
| **T1: Gesture** | 200ms-1s | Tab switch scan line, pane focus shift pulse, scroll phosphor ghosts |
| **T2: Passage** | 0.5-3s | Modal materialization, window switch with crossfade/slide/dissolve |
| **T3: Moment** | 2-8s | Trade pulse, phase transition overlay, perspective awakening |
| **T4: Cinematic** | 5-15s | First-time events, death, birth, achievement unlock |

The tab switch specifically should use one of six transition types: **Crossfade, HorizontalSlide, VerticalDissolve, RadialWipe, GlitchCut, FadeThrough** -- selected based on cognitive state and modulated by the agent's emotional register.

**Gap: Total. None of T0-T4 exist. Every state change is instantaneous.**

---

## 3. What Should Animate That Doesn't

### Critical gaps (high impact, immediately noticeable)

| Element | Current | Should Be |
|---|---|---|
| **Tab switching** | Hard cut | Directional sweep/crossfade (200-400ms) |
| **Modal appear** | Instant pop | Scale-up from center + background dim transition (150-300ms) |
| **Modal dismiss** | Instant vanish | Scale-down + fade-out (100-200ms) |
| **Notification toast** | Instant appear/disappear | Slide-in from right, fade-out on expiry |
| **Progress bars** | Jump to new value | Smooth interpolation toward target (~8 frames) |
| **Sparkline data** | Instant replace | Scroll new data in from right edge |
| **Focus change** | Border color swap | Border pulse/glow that fades over 200ms |

### Important gaps (noticeable during extended use)

| Element | Current | Should Be |
|---|---|---|
| **Scroll** | Instant jump | Smooth scroll with momentum/deceleration |
| **Panel resize** | Instant reflow | Content slides to new position over 2-3 frames |
| **Task status change** | Icon swap | Icon morphs with brief color flash |
| **Plan completion** | Status text change | Completion pulse radiating outward |
| **Error state** | Red text appears | Screen edge flash + brief shake (2-3 frame +-1 cell jitter) |
| **Agent start/stop** | Row appears/disappears | Fade-in/fade-out over 200ms |
| **Gate pass/fail** | Text update | Pass: green sweep. Fail: red flash + shake. |

### Atmospheric gaps (felt as "aliveness" deficit)

| Element | Current | Should Be |
|---|---|---|
| **Noise floor** | None | Sparse random `░▒·∙` shimmer at 0.3-2.0% density |
| **Scanlines** | None | Every 3rd row bg dimmed by 0.04 |
| **Phosphor persistence** | None | Previous frame ghost at 30% opacity |
| **Color breathing** | Only in postfx bg | All border colors should drift +-3 hue units over 10-30s |
| **Cursor glow** | None | Selected row should emit faint radial light |

---

## 4. Effects Preset System Evaluation

### Current mapping

```
effects_config.rs:172  apply_preset():

Off:
  nerv_viz = false
  particles = false
  (screen_postfx set to false by load_from_root)

Minimal (default):
  nerv_viz = false
  particles = true
  (screen_postfx = true)

Full:
  nerv_viz = true
  particles = true
  (screen_postfx = true)
```

### What each level actually delivers

| Feature | Off | Minimal | Full |
|---|---|---|---|
| Self-glow (tabs 0/1/2) | No | Yes | Yes |
| Bloom | No | If `bloom_enabled` | If `bloom_enabled` |
| Drop shadow | No | If `shadows_enabled` | If `shadows_enabled` |
| Ambient orbs | No | If `vfx_enabled` | If `vfx_enabled` |
| Dream atmosphere | No | If `vfx_enabled` | If `vfx_enabled` |
| State viz (bg waves) | No | No | Yes |
| Particles | No | Yes (when agents active) | Yes (when agents active) |
| Modal glow | No | Yes | Yes |
| Data rain | No | No | Rendered by state_viz path |
| Activity ripples | No | No | Rendered by state_viz path |

### Problem: `bloom_enabled`, `shadows_enabled`, `vfx_enabled` are never set to true

The preset `apply_preset()` method (line 172) only touches `nerv_viz` and `particles`. It never sets `bloom_enabled`, `shadows_enabled`, or `vfx_enabled`. Those flags stay at their `Default` values of `false`. This means:

- **Bloom never runs** -- the full bloom pass in `postfx.rs:639` is dead code at runtime
- **Drop shadows never render** -- `postfx.rs:946` is dead code
- **Ambient orbs and dream atmosphere never run** -- `postfx.rs:845` and `postfx.rs:892` are dead code
- **Vignette** -- called directly by `dream_atmosphere` so also dead

The `EffectsConfig` struct has the fields and the pipeline checks them, but the presets never enable them. This is a wiring gap. The "Full" preset should set `bloom_enabled = true`, `vfx_enabled = true`, and `shadows_enabled = true`.

### Proposed preset revision

```
Off:
  Everything false. ROKO_REDUCED_MOTION forces this.

Minimal:
  screen_postfx = true
  particles = true
  (no bloom, no shadows, no vfx -- clean and fast)

Full:
  screen_postfx = true
  nerv_viz = true
  particles = true
  bloom_enabled = true      // NEW
  shadows_enabled = true    // NEW
  vfx_enabled = true        // NEW
  bloom_intensity = 0.15
  vignette_intensity = 0.20

Ultra (proposed new tier):
  Everything in Full, plus:
  scanlines, phosphor persistence, noise floor, chromatic aberration
  (needs additional flags or a separate "cinematic" overlay)
```

The `ROKO_REDUCED_MOTION` env-var escape hatch is well-implemented and should remain as-is.

---

## 5. Proposal: Smooth Tab Transitions

### Architecture

Tab transitions require retaining the previous frame's buffer content for blending. ratatui's `Terminal` already double-buffers (current vs. previous), but only for diff-based rendering. We need an explicit transition buffer.

### Implementation sketch

```rust
// New file: crates/roko-cli/src/tui/transition.rs

use std::time::Instant;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

/// The six transition types from the Mori spec.
#[derive(Debug, Clone, Copy)]
pub enum TransitionKind {
    /// Content crossfades in place -- old fades out, new fades in.
    Crossfade,
    /// Horizontal scan line sweeps in direction of navigation.
    HorizontalSlide,
    /// Vertical dissolve from top or bottom.
    VerticalDissolve,
    /// Radial wipe from center outward.
    RadialWipe,
    /// Glitch artifacts (for error/terminal states).
    GlitchCut,
    /// Fade to black, then fade in (for distant tab jumps).
    FadeThrough,
}

/// Active transition state.
pub struct Transition {
    kind: TransitionKind,
    /// Direction: positive = forward (left-to-right), negative = backward.
    direction: i8,
    started: Instant,
    duration: Duration,
    /// Snapshot of the old tab's rendered buffer content.
    old_buffer: Buffer,
    /// Area the transition covers.
    area: Rect,
}

impl Transition {
    pub fn new(
        kind: TransitionKind,
        direction: i8,
        duration: Duration,
        area: Rect,
        old_buffer: Buffer,
    ) -> Self {
        Self {
            kind,
            direction,
            started: Instant::now(),
            duration,
            old_buffer,
            area,
        }
    }

    /// Progress from 0.0 to 1.0.
    pub fn progress(&self) -> f64 {
        let elapsed = self.started.elapsed().as_secs_f64();
        let total = self.duration.as_secs_f64();
        (elapsed / total).clamp(0.0, 1.0)
    }

    /// Whether the transition has completed.
    pub fn is_done(&self) -> bool {
        self.started.elapsed() >= self.duration
    }

    /// Blend the old buffer into the current (new) buffer.
    pub fn apply(&self, new_buf: &mut Buffer) {
        let t = self.progress();
        let t = ease_out_cubic(t); // smooth deceleration

        match self.kind {
            TransitionKind::Crossfade => {
                self.apply_crossfade(new_buf, t);
            }
            TransitionKind::HorizontalSlide => {
                self.apply_horizontal_slide(new_buf, t);
            }
            TransitionKind::FadeThrough => {
                self.apply_fade_through(new_buf, t);
            }
            TransitionKind::VerticalDissolve => {
                self.apply_vertical_dissolve(new_buf, t);
            }
            TransitionKind::RadialWipe => {
                self.apply_radial_wipe(new_buf, t);
            }
            TransitionKind::GlitchCut => {
                self.apply_glitch_cut(new_buf, t);
            }
        }
    }

    fn apply_crossfade(&self, new_buf: &mut Buffer, t: f64) {
        // For each cell: blend old_buffer color with new_buffer color
        // at ratio (1-t) old + t new.
        for y in self.area.top()..self.area.bottom() {
            for x in self.area.left()..self.area.right() {
                if let (Some(old_cell), Some(new_cell)) =
                    (self.old_buffer.cell((x, y)), new_buf.cell_mut((x, y)))
                {
                    // Blend foreground
                    if let (Some(old_fg), Some(new_fg)) =
                        (extract_rgb(old_cell.fg), extract_rgb(new_cell.fg))
                    {
                        new_cell.set_fg(lerp_color(old_fg, new_fg, t));
                    }
                    // Blend background
                    if let (Some(old_bg), Some(new_bg)) =
                        (extract_rgb(old_cell.bg), extract_rgb(new_cell.bg))
                    {
                        new_cell.set_bg(lerp_color(old_bg, new_bg, t));
                    }
                    // Symbol: show old symbol until midpoint, then new
                    if t < 0.5 {
                        new_cell.set_symbol(old_cell.symbol());
                    }
                }
            }
        }
    }

    fn apply_horizontal_slide(&self, new_buf: &mut Buffer, t: f64) {
        // The "scan line" effect: a vertical line sweeps across the screen.
        // To the left of the line: new content. To the right: old content.
        // (Reversed if direction is negative.)
        let sweep_x = if self.direction >= 0 {
            self.area.left() + (self.area.width as f64 * t) as u16
        } else {
            self.area.right() - (self.area.width as f64 * t) as u16
        };

        for y in self.area.top()..self.area.bottom() {
            for x in self.area.left()..self.area.right() {
                let show_old = if self.direction >= 0 {
                    x >= sweep_x
                } else {
                    x < sweep_x
                };
                if show_old {
                    if let (Some(old_cell), Some(new_cell)) =
                        (self.old_buffer.cell((x, y)), new_buf.cell_mut((x, y)))
                    {
                        *new_cell = old_cell.clone();
                    }
                }
                // At the sweep edge: bright scan line (2-3 cells wide)
                let dist = (x as i32 - sweep_x as i32).unsigned_abs();
                if dist <= 2 {
                    if let Some(cell) = new_buf.cell_mut((x, y)) {
                        let boost = 1.0 - (dist as f64 / 3.0);
                        // Additive brighten by boost amount
                        brighten_cell(cell, (40.0 * boost) as u8);
                    }
                }
            }
        }
    }

    fn apply_fade_through(&self, new_buf: &mut Buffer, t: f64) {
        // First half: fade old to black. Second half: fade black to new.
        if t < 0.5 {
            let fade = 1.0 - (t * 2.0); // 1.0 -> 0.0
            for y in self.area.top()..self.area.bottom() {
                for x in self.area.left()..self.area.right() {
                    if let (Some(old_cell), Some(new_cell)) =
                        (self.old_buffer.cell((x, y)), new_buf.cell_mut((x, y)))
                    {
                        *new_cell = old_cell.clone();
                        dim_cell(new_cell, fade);
                    }
                }
            }
        } else {
            let reveal = (t - 0.5) * 2.0; // 0.0 -> 1.0
            for y in self.area.top()..self.area.bottom() {
                for x in self.area.left()..self.area.right() {
                    if let Some(cell) = new_buf.cell_mut((x, y)) {
                        dim_cell(cell, reveal);
                    }
                }
            }
        }
    }

    // ... VerticalDissolve, RadialWipe, GlitchCut follow same pattern
}

/// Cubic ease-out: fast start, smooth deceleration.
fn ease_out_cubic(t: f64) -> f64 {
    1.0 - (1.0 - t).powi(3)
}
```

### Integration into `app.rs`

```rust
// In App struct:
active_transition: Option<Transition>,

// On tab switch (in handle_action):
TuiAction::SwitchTab(new_tab) => {
    if new_tab != self.tui_state.active_tab {
        let old_idx = self.tui_state.active_tab.index();
        let new_idx = new_tab.index();
        let direction = if new_idx > old_idx { 1 } else { -1 };

        // Capture current buffer as snapshot
        let snapshot = frame.buffer().clone();

        let kind = select_transition_kind(direction, &self.tui_state);
        self.active_transition = Some(Transition::new(
            kind,
            direction,
            Duration::from_millis(300),
            content_area,
            snapshot,
        ));
        self.tui_state.active_tab = new_tab;
    }
}

// In draw loop, after rendering new tab content:
if let Some(ref transition) = self.active_transition {
    if transition.is_done() {
        self.active_transition = None;
    } else {
        transition.apply(frame.buffer_mut());
    }
}
```

### Transition selection heuristic

```rust
fn select_transition_kind(direction: i8, state: &TuiState) -> TransitionKind {
    // Adjacent tabs: horizontal slide (feels like swiping)
    // 2-3 tabs apart: crossfade (quick blend)
    // 4+ tabs apart: fade-through (distance emphasizes the jump)
    // Error state: glitch cut
    if state.has_errors() {
        TransitionKind::GlitchCut
    } else {
        match direction.unsigned_abs() {
            0..=1 => TransitionKind::HorizontalSlide,
            2..=3 => TransitionKind::Crossfade,
            _ => TransitionKind::FadeThrough,
        }
    }
}
```

### Duration: 250-350ms

Fast enough to not feel sluggish (games target 200-300ms for menu transitions), slow enough to register as intentional motion. The spec says 200ms-1s for Tier 1 gestures.

---

## 6. Proposal: Modal Appear/Disappear Animations

### Current behavior

Modals pop in instantly (`render_modal` is called, content appears). When dismissed, the `active_modal` is set to `None` and the modal vanishes on the next frame.

### Proposed: animated modal lifecycle

```rust
// New struct for modal animation state
pub struct ModalAnimation {
    phase: ModalPhase,
    started: Instant,
    /// Duration of the appear/disappear animation.
    duration: Duration,
    /// Target area (final size when fully open).
    target_area: Rect,
}

pub enum ModalPhase {
    Appearing,
    Visible,
    Disappearing,
}

impl ModalAnimation {
    pub fn progress(&self) -> f64 {
        let t = self.started.elapsed().as_secs_f64()
            / self.duration.as_secs_f64();
        t.clamp(0.0, 1.0)
    }

    /// Current render area, interpolated from center to full size.
    pub fn current_area(&self) -> Rect {
        let t = match self.phase {
            ModalPhase::Appearing => ease_out_back(self.progress()),
            ModalPhase::Visible => 1.0,
            ModalPhase::Disappearing => 1.0 - ease_in_cubic(self.progress()),
        };

        let cx = self.target_area.x + self.target_area.width / 2;
        let cy = self.target_area.y + self.target_area.height / 2;
        let w = (self.target_area.width as f64 * t) as u16;
        let h = (self.target_area.height as f64 * t) as u16;
        Rect::new(
            cx.saturating_sub(w / 2),
            cy.saturating_sub(h / 2),
            w.max(1),
            h.max(1),
        )
    }

    /// Background dim factor (0.0 = no dim, 1.0 = full dim).
    pub fn dim_factor(&self) -> f64 {
        let t = self.progress();
        match self.phase {
            ModalPhase::Appearing => t * 0.5,
            ModalPhase::Visible => 0.5,
            ModalPhase::Disappearing => (1.0 - t) * 0.5,
        }
    }
}

/// Ease-out-back: slight overshoot then settle (bouncy feel).
fn ease_out_back(t: f64) -> f64 {
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}
```

### Render integration

```rust
// In render_modals:
if let Some(ref anim) = self.modal_animation {
    let dim = anim.dim_factor();
    postfx::dim_overlay(area, frame.buffer_mut(), 1.0 - dim);

    let render_area = anim.current_area();
    if render_area.width >= 6 && render_area.height >= 3 {
        // Clip modal content to animated area
        render_modal(frame, render_area, modal, tui_state, data, theme);
    }

    if anim.phase == ModalPhase::Appearing && anim.progress() >= 1.0 {
        self.modal_animation.as_mut().unwrap().phase = ModalPhase::Visible;
    }
}
```

### Duration: 200ms appear, 150ms disappear

Disappear should be faster than appear -- the user wants the modal gone NOW. The slight overshoot on appear (`ease_out_back`) gives a physical, springy feel. Disappear uses `ease_in_cubic` for quick acceleration away.

---

## 7. Proposal: Progress Bar Smooth Interpolation

### Current behavior

Progress bars in `task_progress.rs` compute `fill_pct = done / total` and render that many filled cells. When a task completes, the bar jumps instantly.

### Proposed: animated progress with SmoothedValue

The infrastructure already exists in `smoothing.rs`. The `SmoothedValue` struct does EMA with configurable alpha. However, it is not used for progress bars.

```rust
// In TuiState, add:
pub task_progress_smooth: SmoothedValue,
pub plan_progress_smooth: SmoothedValue,

// On state update:
let raw_progress = done as f64 / total.max(1) as f64;
self.task_progress_smooth.update(raw_progress);

// In semantic_bar rendering:
let fill_pct = state.task_progress_smooth.get(); // smoothed, not raw
```

### Tuning

- **Alpha = 0.15** (slightly faster than the default 0.12) -- progress should feel responsive but not jumpy
- **On completion (1.0):** snap immediately, do not smooth the final transition. Completion should feel decisive.
- **On reset (0.0):** snap immediately. New plan should start clean.

```rust
pub fn update_progress(&mut self, raw: f64) {
    if raw >= 1.0 || raw <= 0.0 {
        // Snap on completion or reset
        self.value = raw;
    } else {
        self.value = self.alpha * raw + (1.0 - self.alpha) * self.value;
    }
}
```

### Visual enhancement: leading-edge glow

When the bar is advancing, the leading filled cell gets a brief brightness pulse:

```rust
if filled > 0 && fill_pct < 1.0 {
    // The last filled cell gets extra brightness
    let leading_boost = (elapsed * 3.0).sin() * 0.15 + 0.85;
    let leading_color = brighten(bar_color, leading_boost);
    spans.push(Span::styled("█", Style::default().fg(leading_color)));
    spans.push(Span::styled("█".repeat(filled - 1), Style::default().fg(bar_color)));
} else {
    spans.push(Span::styled("█".repeat(filled), Style::default().fg(bar_color)));
}
```

---

## 8. Proposal: Notification Toast Slide-In/Out

### Current behavior

Toasts appear instantly at the bottom-right corner (`notification.rs:90`) and vanish after TTL expires. No entrance or exit animation.

### Proposed: slide + fade

```rust
pub struct Notification {
    pub message: String,
    pub created: Instant,
    pub ttl_secs: u64,
    pub level: NotificationLevel,
}

impl Notification {
    /// Animation progress for entrance (0.0 = offscreen right, 1.0 = settled).
    pub fn entrance_progress(&self) -> f64 {
        let age = self.created.elapsed().as_secs_f64();
        ease_out_cubic((age / 0.3).clamp(0.0, 1.0)) // 300ms entrance
    }

    /// Animation progress for exit (0.0 = visible, 1.0 = gone).
    pub fn exit_progress(&self) -> f64 {
        let remaining = self.ttl_secs as f64
            - self.created.elapsed().as_secs_f64();
        if remaining <= 0.5 {
            // Last 500ms: fade out
            ease_in_cubic(1.0 - (remaining / 0.5).clamp(0.0, 1.0))
        } else {
            0.0
        }
    }

    /// X offset for slide animation (0 = final position, positive = offscreen right).
    pub fn x_offset(&self) -> u16 {
        let entrance = 1.0 - self.entrance_progress();
        let exit = self.exit_progress();
        let offset = (entrance.max(exit) * 20.0) as u16;
        offset
    }

    /// Opacity factor (1.0 = fully visible, 0.0 = invisible).
    pub fn opacity(&self) -> f64 {
        let entrance = self.entrance_progress();
        let exit = 1.0 - self.exit_progress();
        entrance.min(exit)
    }
}
```

### Render changes

```rust
pub fn render_notifications(/* ... */) {
    for (i, notif) in visible.iter().enumerate() {
        let x_slide = notif.x_offset();
        let opacity = notif.opacity();

        if opacity <= 0.05 {
            continue; // fully faded out
        }

        let toast_area = Rect::new(
            area.x + x_offset + x_slide, // slide from right
            area.y + y_offset,
            toast_width.min(area.width.saturating_sub(x_slide)),
            toast_height,
        );

        frame.render_widget(Clear, toast_area);
        // ... render block and paragraph ...

        // Apply opacity as color dimming
        if opacity < 1.0 {
            postfx::dim_overlay(toast_area, frame.buffer_mut(), opacity);
        }
    }
}
```

---

## 9. Proposal: Cursor/Selection Glow and Pulse Effects

### Current behavior

Selected rows get `BG_HIGHLIGHT` background and `BOLD` modifier. No glow, no pulse, no radial light.

### Proposed: selection glow

```rust
/// Render a soft glow around the selected row.
pub fn selection_glow(area: Rect, buf: &mut Buffer, selected_y: u16, elapsed: f64) {
    let pulse = (elapsed * 2.5).sin() * 0.3 + 0.7; // 0.4..1.0 range

    for y in area.top()..area.bottom() {
        let dist = (y as i32 - selected_y as i32).unsigned_abs();
        if dist > 3 { continue; }

        let falloff = match dist {
            0 => 1.0 * pulse,
            1 => 0.4 * pulse,
            2 => 0.15 * pulse,
            3 => 0.05 * pulse,
            _ => 0.0,
        };

        for x in area.left()..area.right() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                if let Some((r, g, b)) = cell_bg_rgb(cell) {
                    // Warm rose tint
                    let nr = (r as f64 + 30.0 * falloff).min(255.0) as u8;
                    let ng = (g as f64 + 10.0 * falloff).min(255.0) as u8;
                    let nb = (b as f64 + 18.0 * falloff).min(255.0) as u8;
                    cell.set_bg(Color::Rgb(nr, ng, nb));
                }
            }
        }
    }
}
```

This creates a soft vertical glow centered on the selected row -- 3 rows of falloff above and below, pulsing with the heartbeat. The effect is subtle: at most +30 red channel on the selected row itself, fading to +1-2 on the periphery.

---

## 10. Proposal: Background/Scanline/CRT Effects (NERV Aesthetic)

### Scanlines

From the spec: "darken every 3rd row by 0.04." This is the simplest atmospheric effect and should be in the Minimal preset.

```rust
/// CRT-style scanline dimming. Every `spacing`-th row gets darkened.
pub fn scanlines(area: Rect, buf: &mut Buffer, spacing: u16, darken_amount: f64) {
    let darken = darken_amount.clamp(0.0, 0.2); // safety cap
    for y in (area.top()..area.bottom()).step_by(spacing as usize) {
        for x in area.left()..area.right() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                if let Some((r, g, b)) = cell_bg_rgb(cell) {
                    let factor = 1.0 - darken;
                    cell.set_bg(Color::Rgb(
                        (r as f64 * factor) as u8,
                        (g as f64 * factor) as u8,
                        (b as f64 * factor) as u8,
                    ));
                }
                if let Some((r, g, b)) = cell_fg_rgb(cell) {
                    let factor = 1.0 - darken * 0.5; // less effect on fg
                    cell.set_fg(Color::Rgb(
                        (r as f64 * factor) as u8,
                        (g as f64 * factor) as u8,
                        (b as f64 * factor) as u8,
                    ));
                }
            }
        }
    }
}
```

### Phosphor persistence (ghost frames)

Requires maintaining the previous frame's buffer. On each frame, blend the previous buffer at low opacity into the current one before rendering.

```rust
/// Blend previous-frame content as a ghost layer.
pub fn phosphor_persist(
    area: Rect,
    current: &mut Buffer,
    previous: &Buffer,
    opacity: f64,
) {
    let opacity = opacity.clamp(0.0, 0.4); // never more than 40%
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let (Some(prev), Some(curr)) =
                (previous.cell((x, y)), current.cell_mut((x, y)))
            {
                // Only ghost non-matching content
                if prev.symbol() != curr.symbol() {
                    if let Some((pr, pg, pb)) = cell_fg_rgb(prev) {
                        if let Some((cr, cg, cb)) = cell_fg_rgb(curr) {
                            let nr = lerp_u8(cr, pr, opacity);
                            let ng = lerp_u8(cg, pg, opacity);
                            let nb = lerp_u8(cb, pb, opacity);
                            curr.set_fg(Color::Rgb(nr, ng, nb));
                        }
                    }
                }
            }
        }
    }
}
```

### Noise floor

From the spec: 0.3-2.0% of blank cells get a random dim character per frame.

```rust
/// Sparse noise floor -- shimmer in blank cells.
pub fn noise_floor(
    area: Rect,
    buf: &mut Buffer,
    density: f64,  // 0.003 to 0.02
    frame_seed: u64,
    warm: bool,    // warm (rose) vs cool (indigo)
) {
    const NOISE_CHARS: &[char] = &['░', '▒', '·', '∙'];

    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let hash = splitmix64(
                frame_seed ^ (x as u64) << 16 ^ (y as u64) << 32
            );
            if (hash as f64 / u64::MAX as f64) > density {
                continue;
            }

            if let Some(cell) = buf.cell_mut((x, y)) {
                if !is_blank(cell) { continue; }

                let ch = NOISE_CHARS[(hash >> 8) as usize % NOISE_CHARS.len()];
                let brightness = 20 + (hash >> 16) as u8 % 20;
                let fg = if warm {
                    Color::Rgb(brightness + 10, brightness / 2, brightness)
                } else {
                    Color::Rgb(brightness / 2, brightness, brightness + 10)
                };
                cell.set_char(ch);
                cell.set_fg(fg);
            }
        }
    }
}
```

### CRT curvature (barrel distortion)

True barrel distortion is impractical in a character grid since you cannot move cells by sub-character amounts. However, a convincing approximation is possible:

- Darken corners more aggressively than vignette (the existing `vignette()` already does this)
- Add 1-cell horizontal indent on the top and bottom 2 rows (simulating screen curvature)
- Horizontal color shift of +/-1 cell at extreme edges (chromatic aberration)

This is a "nice to have" for the Ultra preset, not Minimal or Full.

### Preset assignment

| Effect | Off | Minimal | Full | Ultra (proposed) |
|---|---|---|---|---|
| Scanlines | No | Yes (every 3, 0.04) | Yes (every 3, 0.04) | Yes (every 2, 0.06) |
| Noise floor | No | No | Yes (0.3%) | Yes (0.8%) |
| Phosphor persist | No | No | Yes (15%) | Yes (30%) |
| CRT curvature | No | No | No | Yes |
| Chromatic aberration | No | No | No | Yes (edge only) |

---

## 11. ratatui Custom Rendering Capabilities

### What ratatui gives us for animations

1. **Direct `Buffer` manipulation** -- `Widget::render(area, buf)` gives mutable access to every cell. The `postfx.rs` module already demonstrates comprehensive buffer manipulation (bloom, vignette, dim, glow, particles). This is the correct approach.

2. **`Cell` properties available for effects:**
   - `cell.symbol()` -- character content (read/write)
   - `cell.fg` / `cell.bg` -- truecolor RGB (read/write)
   - `cell.modifier` -- Bold, Dim, Italic, Underline, Blink, etc.
   - `cell.set_style(Style)` -- batch update

3. **Double buffering** -- `Terminal` internally diffs current vs. previous buffer and only emits changed cells. This means our animation passes can modify many cells freely; only actual changes hit the terminal. This makes effects performant as long as they are deterministic (non-changing cells produce no output).

4. **No native animation timeline** -- ratatui has no built-in concept of animations, tweens, or easing. All timing must be external (our `Atmosphere` struct handles this).

5. **Frame rate** -- ratatui renders as fast as we call `terminal.draw()`. The event loop in `app.rs` uses `crossterm::event::poll` with a timeout that effectively sets the frame rate. Active animation frames need ~16ms (60fps) or ~33ms (30fps) poll intervals.

6. **Buffer cloning** -- `Buffer::clone()` is available for snapshot-based transitions. A 200x60 terminal is 12,000 cells, each ~40 bytes = ~480KB. This is acceptable for a single transition buffer. Do not hold more than 2 (old + current).

7. **tachyonfx integration** -- The PRD references `tachyonfx` (shader effects for ratatui), but it is not currently a dependency. It provides composable `Shader` and `CellFilter` traits. Worth evaluating but not required -- the existing `postfx.rs` already does everything tachyonfx would provide, just without the composition framework.

### Performance budget

The spec allocates 1.5ms for post-processing. At 60fps that is 16.67ms total frame budget. Current rendering (layout + widgets) takes ~2-8ms depending on tab complexity. That leaves ~8-14ms for effects.

Measured costs of existing effects (estimated from algorithm complexity on 200x60 terminal):
- Scanlines: <0.1ms (skip every 3rd row, simple multiply)
- Self-glow: ~0.3ms (single pass, per-cell luminance check)
- Bloom: ~1-2ms (two passes: collect + apply, O(w*h*radius^2))
- Particles: ~0.1ms (24 particles max, 3x3 check each)
- State viz: ~0.5ms (two nested sin() calls per cell)
- Vignette: ~0.3ms (distance calc per cell)

Total current pipeline: ~2-3ms. Headroom for transitions: ~5-10ms.

### What is NOT possible in ratatui

- **Sub-cell positioning** -- characters are on a fixed grid. No smooth pixel-level sliding.
- **Alpha blending** -- terminals do not support transparency. We fake it with color interpolation.
- **Custom fonts/glyphs** -- limited to Unicode. Braille (U+2800-U+28FF) gives 2x4 sub-cell dots as the highest resolution available.
- **GPU acceleration** -- everything is CPU. Keep effects O(n) where n = cell count.
- **Audio** -- ratatui is visual only. The `rodio` dependency mentioned in the spec is not present and should be separate.

---

## 12. Proposal: Animation Framework (`Animator`)

### Design

A general-purpose animation scheduler that manages multiple concurrent animations, each with its own timeline, easing function, and completion callback.

```rust
// New file: crates/roko-cli/src/tui/animator.rs

use std::time::{Duration, Instant};

/// Easing function type.
pub type EasingFn = fn(f64) -> f64;

// --- Standard easing library ---

pub fn linear(t: f64) -> f64 { t }

pub fn ease_in_quad(t: f64) -> f64 { t * t }
pub fn ease_out_quad(t: f64) -> f64 { t * (2.0 - t) }
pub fn ease_in_out_quad(t: f64) -> f64 {
    if t < 0.5 { 2.0 * t * t } else { -1.0 + (4.0 - 2.0 * t) * t }
}

pub fn ease_in_cubic(t: f64) -> f64 { t * t * t }
pub fn ease_out_cubic(t: f64) -> f64 { 1.0 - (1.0 - t).powi(3) }
pub fn ease_in_out_cubic(t: f64) -> f64 {
    if t < 0.5 { 4.0 * t * t * t }
    else { 1.0 - (-2.0 * t + 2.0).powi(3) / 2.0 }
}

pub fn ease_out_back(t: f64) -> f64 {
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

pub fn ease_out_elastic(t: f64) -> f64 {
    if t <= 0.0 { return 0.0; }
    if t >= 1.0 { return 1.0; }
    let c4 = std::f64::consts::TAU / 3.0;
    2.0_f64.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
}

pub fn ease_out_bounce(t: f64) -> f64 {
    let n1 = 7.5625;
    let d1 = 2.75;
    if t < 1.0 / d1 {
        n1 * t * t
    } else if t < 2.0 / d1 {
        let t = t - 1.5 / d1;
        n1 * t * t + 0.75
    } else if t < 2.5 / d1 {
        let t = t - 2.25 / d1;
        n1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / d1;
        n1 * t * t + 0.984375
    }
}

/// A uniquely identified animation channel.
pub type AnimId = u64;

/// A single running animation.
pub struct Animation {
    pub id: AnimId,
    started: Instant,
    duration: Duration,
    easing: EasingFn,
    /// Start value.
    from: f64,
    /// End value.
    to: f64,
    /// Whether to remove on completion or hold at final value.
    auto_remove: bool,
}

impl Animation {
    /// Raw progress (0.0 to 1.0, linear).
    pub fn raw_progress(&self) -> f64 {
        let elapsed = self.started.elapsed().as_secs_f64();
        let total = self.duration.as_secs_f64();
        if total <= 0.0 { 1.0 } else { (elapsed / total).clamp(0.0, 1.0) }
    }

    /// Eased progress.
    pub fn progress(&self) -> f64 {
        (self.easing)(self.raw_progress())
    }

    /// Current interpolated value.
    pub fn value(&self) -> f64 {
        let t = self.progress();
        self.from + (self.to - self.from) * t
    }

    /// Whether the animation has finished.
    pub fn is_done(&self) -> bool {
        self.started.elapsed() >= self.duration
    }
}

/// Central animation scheduler.
pub struct Animator {
    animations: Vec<Animation>,
    next_id: AnimId,
}

impl Animator {
    pub fn new() -> Self {
        Self {
            animations: Vec::new(),
            next_id: 1,
        }
    }

    /// Start a new animation. Returns its ID for later querying.
    pub fn animate(
        &mut self,
        from: f64,
        to: f64,
        duration: Duration,
        easing: EasingFn,
    ) -> AnimId {
        let id = self.next_id;
        self.next_id += 1;
        self.animations.push(Animation {
            id,
            started: Instant::now(),
            duration,
            easing,
            from,
            to,
            auto_remove: true,
        });
        id
    }

    /// Get the current value of an animation by ID.
    /// Returns `None` if the animation has completed and been removed.
    pub fn value(&self, id: AnimId) -> Option<f64> {
        self.animations
            .iter()
            .find(|a| a.id == id)
            .map(|a| a.value())
    }

    /// Get value or return a default if the animation is gone.
    pub fn value_or(&self, id: AnimId, default: f64) -> f64 {
        self.value(id).unwrap_or(default)
    }

    /// Check if an animation is still running.
    pub fn is_active(&self, id: AnimId) -> bool {
        self.animations.iter().any(|a| a.id == id && !a.is_done())
    }

    /// Prune completed animations. Call once per frame.
    pub fn tick(&mut self) {
        self.animations
            .retain(|a| !a.auto_remove || !a.is_done());
    }

    /// Cancel an animation.
    pub fn cancel(&mut self, id: AnimId) {
        self.animations.retain(|a| a.id != id);
    }

    /// Number of active animations.
    pub fn active_count(&self) -> usize {
        self.animations.iter().filter(|a| !a.is_done()).count()
    }
}
```

### Usage examples

```rust
// Modal appear:
let anim_id = animator.animate(0.0, 1.0, Duration::from_millis(200), ease_out_back);

// In render:
let scale = animator.value_or(anim_id, 1.0);
let modal_width = (target_width as f64 * scale) as u16;
let modal_height = (target_height as f64 * scale) as u16;

// Progress bar smooth advance:
let bar_anim = animator.animate(
    old_progress, new_progress,
    Duration::from_millis(400),
    ease_out_cubic,
);

// Tab transition:
let sweep = animator.animate(0.0, 1.0, Duration::from_millis(300), ease_out_cubic);

// Notification slide:
let slide = animator.animate(30.0, 0.0, Duration::from_millis(300), ease_out_cubic);

// Selection glow pulse (looping -- use a separate oscillator, not Animator):
// Animator is for finite-duration transitions. Use Atmosphere for continuous oscillation.
```

### Integration into App

```rust
pub struct App {
    // ... existing fields ...
    animator: Animator,
}

// In the draw loop:
fn draw(&mut self, frame: &mut Frame) {
    self.animator.tick(); // prune completed animations

    // Use animator values for all animated properties
    // ...
}
```

### Design principles

1. **Animator is for finite transitions.** It manages start-to-end animations with a defined duration. Use `Atmosphere` for continuous oscillations (heartbeat, breathing, spinner).

2. **One value per animation.** Complex multi-property animations (like a modal that scales AND fades AND slides) use multiple AnimId values queried independently.

3. **Auto-remove by default.** Completed animations are pruned on the next `tick()`. This prevents memory growth from thousands of tiny animations.

4. **No allocation per frame.** The `Vec<Animation>` grows once per animation start and shrinks on prune. No per-frame allocation.

5. **Thread-safe by design.** The Animator lives on the main thread alongside the render loop. No synchronization needed.

---

## Implementation Priority

### Phase 1: Foundation (game-feel baseline)

1. **Fix effects presets** -- `apply_preset(Full)` should set `bloom_enabled`, `shadows_enabled`, `vfx_enabled` to true. One-line fix that unlocks 3 dormant effects.
2. **Add `Animator` struct** -- the easing function library and animation scheduler. Pure data, no rendering dependencies.
3. **Add scanlines** to `postfx.rs` and enable in Minimal preset. Single pass, <0.1ms.
4. **Add noise floor** to `postfx.rs` and enable in Full preset.

### Phase 2: Transitions (the biggest gap)

5. **Add `Transition` struct** with buffer snapshot approach.
6. **Wire tab transitions** -- `HorizontalSlide` for adjacent tabs, `Crossfade` for distant.
7. **Wire modal animations** -- scale-up on appear, scale-down on dismiss.
8. **Wire notification slide-in/out** -- horizontal slide from right + opacity fade.

### Phase 3: Polish (interactive feel)

9. **Smooth progress bars** using `SmoothedValue` or `Animator`.
10. **Selection glow** as a postfx pass after widget rendering.
11. **Focus-change pulse** -- border brightness pulse on tab/pane focus change.

### Phase 4: Atmospheric (NERV aesthetic)

12. **Phosphor persistence** -- requires previous-frame buffer retention.
13. **Color breathing** -- slow hue drift on border colors.
14. **CRT effects** -- barrel distortion approximation, chromatic aberration at edges.
15. **State-driven transition intensity** -- transitions modulated by agent state/affect.

### Cost estimates

| Item | Code size | Risk | Impact |
|---|---|---|---|
| Fix presets | 5 lines | None | Unlocks bloom/shadow/orbs |
| Animator struct | ~200 lines | None | Foundation for all animations |
| Scanlines | ~20 lines | None | Instant atmospheric upgrade |
| Noise floor | ~30 lines | None | Ambient aliveness |
| Tab transitions | ~300 lines | Medium (buffer snapshot perf) | Highest single-item UX impact |
| Modal animations | ~150 lines | Low | Professional feel |
| Notification slide | ~50 lines | Low | Polish |
| Progress smooth | ~20 lines | None | Removes visual jarring |
| Selection glow | ~40 lines | None | Spatial grounding |
| Phosphor persist | ~60 lines | Low (memory for old buffer) | CRT feel |

---

## Cross-references

- **Transition spec:** `/Users/will/dev/uniswap/bardo/prd/18-interfaces/rendering/03-transitions.md` -- five-tier system, six transition types, novelty engine, PAD modulation
- **NERV aesthetic:** `/Users/will/dev/uniswap/bardo/prd/18-interfaces/rendering/04-nerv-aesthetic.md` -- institutional register, unit arrays, waveform traces, tactical displays
- **Demoscene algorithms:** `/Users/will/dev/uniswap/bardo/prd/18-interfaces/rendering/01-demoscene.md` -- plasma, fire, tunnel, metaballs, particle systems, braille sub-pixel rendering
- **Effects config:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/effects_config.rs` -- preset system, TOML persistence, reduced-motion
- **PostFX module:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/postfx.rs` -- bloom, vignette, dim, glow, particles, data rain, state viz
- **PostFX pipeline:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/postfx_pipeline.rs` -- per-tab dispatch, viz context construction
- **Atmosphere:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/atmosphere.rs` -- timing, heartbeat, breathing, spinners
- **Smoothing:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/smoothing.rs` -- EMA for display values

---

## Implementation Status (2026-09-02)

### Completed

| Item | What was done | Files |
|---|---|---|
| **1. Fix effects presets** | `apply_preset(Full)` now sets `bloom_enabled`, `shadows_enabled`, `vfx_enabled` to true; test updated | `effects_config.rs` |
| **3. Scanlines** | `scanlines()` added to postfx.rs; wired into pipeline (always-on when postfx enabled, spacing=3, darken=4%); 1 test | `postfx.rs`, `postfx_pipeline.rs` |
| **4. Noise floor** | `noise_floor()` added to postfx.rs; wired into pipeline (Full preset only, density=0.003); rose-tinted sparse shimmer in blank cells; 1 test | `postfx.rs`, `postfx_pipeline.rs` |
| **6. Tab transitions (simplified)** | Fade-in overlay on tab switch (200ms ease-out cubic) instead of full buffer-snapshot approach; `tab_transition` field on App | `app.rs`, `postfx.rs` |
| **8. Notification fade** | `opacity()` method on Notification with 300ms entrance / 500ms exit fade; `fade_overlay()` applied per-toast | `notification.rs`, `postfx.rs` |
| **9. Progress bar smoothing** | Leading-edge glow on `semantic_bar` pulsed by heartbeat; `SmoothedValue::update_progress()` snaps at 0/1 boundaries | `task_progress.rs`, `smoothing.rs`, `state.rs` |
| **Active agent spinners** | `status_icon_animated()` uses `atmosphere.spinner()` braille rotation for Active agents in both full and compact grid | `agent_status_grid.rs` |

### Remaining

| Item | Status | Notes |
|---|---|---|
| **2. Animator struct** | Not built | Full animation scheduler was not needed; individual animations use simpler inline approaches |
| **5. Transition struct (buffer snapshot)** | Skipped | Fade-in approach chosen over buffer-snapshot for simplicity; covers the biggest UX gap |
| **7. Modal animations** | Not done | Scale-up/down on modal appear/dismiss |
| **10. Selection glow** | Not done | PostFX pass for focused/selected items |
| **11. Focus-change pulse** | Not done | Border brightness pulse on focus change |
| **12-15. Atmospheric (Phase 4)** | Not done | Phosphor persistence, color breathing, CRT barrel distortion, state-driven intensity |
