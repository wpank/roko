# Bardo Visual Design Bible -- Roko TUI Reference

**Compiled from:** 9 bardo PRD documents (18-interfaces layer)
**Purpose:** Unified reference for roko TUI visual improvement
**Date:** 2026-09-01

---

## Context: What Bardo Was

Bardo was an autonomous DeFi agent toolkit (the predecessor project to roko) with an
extremely ambitious TUI built on ratatui 0.29 / crossterm 0.28. The TUI was a 60fps
full-screen terminal application connected to a "Golem" (an autonomous agent) via WebSocket.
Each agent had a procedurally generated dot-cloud creature called a "Spectre" that visually
represented its emotional and health state in real time.

The visual system was organized across three crates: `bardo-terminal` (main binary),
`bardo-sprites` (creature sprite engine), and `bardo-tui-widgets` (33 custom widgets). The
design language was called **ROSEDUST** -- rose on violet-black, seen through dirty CRT glass.

Roko inherits the architecture goals but has different domain specifics (plan execution, code
generation, and self-hosting rather than DeFi trading). This document extracts the visual
principles, widget designs, effects, and interaction patterns that apply to roko's TUI.

---

## 1. VISUAL DESIGN PRINCIPLES

### 1.1 The Perpetual Motion Principle

> "Nothing on screen is ever at rest."

Every element is driven by at least one continuously changing variable. A label that never
changes still sits on a background that shimmers. A border that never moves still dims with
lifecycle degradation. A number that hasn't updated still has a phosphor afterimage of its
last change fading behind it. **Static pixels are bugs.**

For every element on every screen, three questions must be answered:

1. **What makes it move?** Which state channel(s) drive this element?
2. **What makes it change?** What state transitions alter its appearance?
3. **What makes it decay?** How does this element show the passage of time?

### 1.2 The 7 Rendering Laws

These are non-negotiable design constraints:

**Law 1: Light follows significance.** The terminal is dark by default. Brightness is earned
by importance. A routine tick is nearly invisible. An important event blazes. The brightness
of any element is proportional to its significance.

**Law 2: Color is state taxonomy.** Every color answers one question: what is this thing's
relationship to the current state? Color is never decorative.

**Law 3: Bold boundaries, soft interiors.** Panel borders are sharp, single-character-width
lines using box-drawing characters. Inside: soft gradients through character density and
opacity variation. Hard geometry (borders) contrasts with soft atmosphere (interiors).

**Law 4: Restraint as aesthetic.** 50% or more of the screen is empty at any given time.
Negative space is not wasted space. A single highlighted number on a field of black has more
impact than a dense dashboard. When in doubt, remove.

**Law 5: Observation costs something.** Where possible, make monitoring costs visible.
Diagnostic views consume resources. This is relevant to roko where inference costs money.

**Law 6: The terminal IS the body.** The agent does not exist behind the terminal. The agent
exists AS the terminal. Scanlines, phosphor bleed, noise -- these are the texture of the
agent's embodiment. Healthy agent, clear display. Struggling agent, degrading display.

**Law 7: Identity is fragile.** The agent's coherence is a continuous achievement, not a
stable property. The display should periodically betray instability. Micro-glitches.
Moments where text speaks in two voices. Frames where layout stutters. These are not
errors -- they are the system's state made visible.

### 1.3 The 5 Meta-Principles

1. **Subtraction reveals more than addition.** Limited palette. Sparse patterns. Less text.
   More void. A Rothko field, not a Kandinsky explosion.

2. **Time is the primary rendering dimension.** Multiple simultaneous timescales run at all
   times: fast flicker (2-3 frames), medium breathing (30-60 frames), slow drift (minutes),
   persistent ghosts (session-spanning). No screenshot fully represents the current state.

3. **The medium must acknowledge itself.** The terminal grid, ANSI codes, Unicode blocks,
   and CRT behavior are visible as the medium. Block elements are not approximations of
   pixels -- they ARE the visual language.

4. **Boundaries are identity; dissolving them is dissolution.** Panel borders and frame
   elements are structural representations of cognitive boundaries. During error states or
   shutdown, dissolving the UI's structural elements IS the visual for system degradation.

5. **Imperfection is the signature of life.** Controlled imperfection (+-1 character jitter,
   slight color variance, timing irregularities) creates the sense of a living presence. Use
   incommensurate sine frequencies (f1=8.3, f2=11.7, f3=15.1, f4=19.9) to produce organic
   quality from pure mathematics -- ratios that never exactly repeat.

### 1.4 The Dual-Reading Principle (NERV Aesthetic)

Every screen should work at two distances:

- **From across the room (1-2 second glance):** The viewer sees the emotional state of the
  interface -- how much of the screen is bright vs. dark, what the dominant color is, whether
  there are crisis elements (flashing, hazard stripes).

- **Up close (sustained reading):** The viewer reads specific numbers, log lines, gauge
  values, status text.

If a screen looks the same from across the room regardless of the system's state, the
atmospheric design has failed.

### 1.5 Density Gradient

Dense edges, sparse interior. Headers, status bars, and border regions carry dense secondary
information. The primary data area breathes. The dense edges hold the emptiness in place:

```
  ████████████████████████████████████████████████████  <- DENSE: header bar
  ████                                          ████
  ████        (primary data: sparse)            ████  <- MARGINS: dense
  ████                                          ████
  ████            0.711                         ████    INTERIOR: sparse
  ████                                          ████
  ████████████████████████████████████████████████████  <- DENSE: status bar
```

---

## 2. COLOR SYSTEM

### 2.1 The ROSEDUST Palette (adapted for roko)

ROSEDUST is monochromatic-dominant. 80% of visible color is rose or its variants. The accent
color appears at most once per screen. White (#FFFFFF) is never used. Pure black (#000000) is
never used.

#### Base and void

| Token | Hex | Usage |
|---|---|---|
| `bg_void` | `#060608` | Deepest background. Nearly black with violet undertone. Never pure black. |
| `bg_raised` | `#0C0A0E` | Panels, containers, raised surfaces |
| `bg_mid` | `#080810` | Intermediate depth. Headers, status bars, overlays |
| `bg_warm` | `#0A0808` | Warm-shifted void for degraded states |
| `border` | `#181420` | Panel borders. Visible but not assertive |
| `border_active` | `#AA708844` | Active panel border. Rose at reduced opacity |

#### Rose spectrum

| Token | Hex | Usage |
|---|---|---|
| `rose` | `#AA7088` | Primary text, headers, active data |
| `rose_bright` | `#CC90A8` | Alerts, danger, high-importance glow |
| `rose_dim` | `#7A5060` | Secondary labels, less important data |
| `rose_deep` | `#3A2030` | Barely visible. Background tints, ghost text |
| `rose_ember` | `#482838` | Phosphor residue. Afterimage of rose |

#### Accent (bone)

| Token | Hex | Usage |
|---|---|---|
| `bone` | `#C8B890` | THE most important element on any screen. Used ONCE per screen, max |
| `bone_dim` | `#8A7A5A` | Dimmed bone. Secondary emphasis within bone-marked elements |

#### Text hierarchy

| Token | Hex | Usage |
|---|---|---|
| `text_primary` | `#988090` | Standard readable text. Cool mauve-grey |
| `text_dim` | `#584858` | Secondary text, labels |
| `text_ghost` | `#302830` | Barely visible. Background murmur, fading content |
| `text_phantom` | `#201820` | Below ghost. Subliminal. Ambient artifacts only |

#### Semantic colors

| Token | Hex | Usage |
|---|---|---|
| `dream` | `#585878` | Altered/special states, connectivity |
| `warning` | `#AA8855` | Amber. Time-related warnings, resource alerts |
| `success` | `#70887A` | Muted sage. Nominal, healthy. Never celebratory |

#### CRT materiality

| Token | Hex | Usage |
|---|---|---|
| `scanline_dark` | `#050507` | Darkened scanline rows |
| `phosphor_res` | `#1A1018` | Ghost of recently-bright pixels |
| `bleed_rose` | `#AA708818` | Simulated phosphor bleed around bright text |
| `noise_warm` | `#2A1820` | Warm-shifted noise for degraded states |
| `noise_cool` | `#201828` | Cool-shifted noise for dream/calm states |

### 2.2 Color Rules

1. 80% rose. The interface is one-color-dominant.
2. Bone appears once per screen. If nothing is critical, bone does not appear at all.
3. The brightest element is `rose_bright` at `#CC90A8`. Never white.
4. Background is `#060608`, not `#000000`. Pure black is a hole; `#060608` has depth.
5. Color transitions are always gradual. Nothing snaps. Everything fades.
6. CRT materiality tokens are never used for content. They are infrastructure.
7. Degradation shifts warm. As the system declines, void moves from violet-black toward
   warm-black (`#0A0808`).

### 2.3 Contrast Ratios

| Level | Pair | Ratio | Purpose |
|---|---|---|---|
| Maximum | `bone` on `bg_void` | ~12:1 | The one number that matters |
| High | `rose_bright` on `bg_void` | ~8:1 | Danger, critical events |
| Primary | `rose` on `bg_void` | ~5:1 | Active data |
| Medium | `text_primary` on `bg_void` | ~3.5:1 | Body text |
| Low | `text_dim` on `bg_void` | ~1.5:1 | Background murmur |
| Ambient | `text_ghost` on `bg_void` | ~1.1:1 | Atmospheric text |

The most powerful design move is a **contrast event**: something normally low-contrast
suddenly becoming high-contrast.

### 2.4 Web UI Colors (for roko-serve dashboard)

For web surfaces (React/HTML), ROSEDUST adapts:

| Token | Value | Usage |
|---|---|---|
| `--black` | `#0A0A0A` | Page background (NOT pure #000000) |
| `--surface` | `#111111` | Elevated sections |
| `--surface-2` | `#161616` | Secondary elevation (sidebar, panels) |
| `--gold` | `#C9A84C` | Primary brand, active states |
| `--gold-dim` | `rgba(201,168,76,0.20)` | Borders at rest |
| `--gold-glow` | `rgba(201,168,76,0.05)` | Hover backgrounds |

Glass morphism levels:

| Level | Background | Border | Blur |
|---|---|---|---|
| Glass-1 (Subtle) | `rgba(255,255,255,0.02)` | `rgba(255,255,255,0.05)` | 12px |
| Glass-2 (Standard) | `rgba(255,255,255,0.04)` | `rgba(255,255,255,0.08)` | 20px |
| Glass-3 (Prominent) | `rgba(255,255,255,0.06)` | `rgba(255,255,255,0.12)` | 30px |

---

## 3. TYPOGRAPHY AND CHARACTER VOCABULARY

### 3.1 TUI Typography

Three "fonts" via Unicode weight:

| Register | Characters | Usage |
|---|---|---|
| System headers | Fullwidth Unicode (U+FF00 block): `ＲＯＫＯ` | Phase names, critical status |
| Body text | Standard ASCII | Data, logs, normal content |
| Data values | Monospace numerics | Numbers, addresses, identifiers |

Fullwidth characters occupy two terminal cells. They read as heavier, more institutional.
Reserved for system headers and critical status. Never use for body text or data values.

```
System headers:  ⌈ ＲＯＫＯ ⌋
Phase names:     ＴＨＲＩＶＩＮＧ, ＴＥＲＭＩＮＡＬ
Critical status: ⌈ ＣＯＮＤＩＴＩＯＮ: ＣＲＩＴＩＣＡＬ ⌋
```

### 3.2 Block Elements (Primary Rendering Primitives)

| Character | Codepoint | Usage |
|---|---|---|
| `█` | U+2588 | Full block. Maximum density, solid fill, gauge bars |
| `▓` | U+2593 | ~75% density, decay chain step 2 |
| `▒` | U+2592 | ~50% density, noise floor, decay step 3 |
| `░` | U+2591 | ~25% density, noise floor, decay step 4 |
| `▀` | U+2580 | Upper half block. Double vertical resolution |
| `▄` | U+2584 | Lower half block. Double vertical resolution |

The half-block technique: `▀` with different fg/bg colors gives two vertical pixels per cell.
An 80x24 terminal becomes 80x48 effective pixels.

### 3.3 Waveform Characters

```
▁ ▂ ▃ ▄ ▅ ▆ ▇ █   (8 levels of vertical fill)
```

Used for sparklines, waveform traces, bar charts. One character per time unit, scrolling left
as new data arrives.

### 3.4 Braille Patterns (Sub-Pixel Precision)

Range: U+2800 to U+28FF (256 characters). Each cell is a 2x4 dot grid (8 sub-pixels). An
80x24 terminal becomes 160x96 effective resolution.

```
Dot layout:
  [0] [3]     Bit 0: top-left        Bit 3: top-right
  [1] [4]     Bit 1: middle-left     Bit 4: middle-right
  [2] [5]     Bit 2: lower-left      Bit 5: lower-right
  [6] [7]     Bit 6: bottom-left     Bit 7: bottom-right

Character code: U+2800 + (b0 | b1<<1 | b2<<2 | b3<<3 | b4<<4 | b5<<5 | b6<<6 | b7<<7)
```

### 3.5 Box-Drawing Characters

| Set | Characters | Usage |
|---|---|---|
| Single line | `- | +` and `┌ ┐ └ ┘ ├ ┤ ┬ ┴ ┼` | Standard panel borders |
| Double line | `═ ║ ╔ ╗ ╚ ╝` | Overlay/modal borders (depth 3) |
| Heavy | `━ ┃ ┏ ┓ ┗ ┛` | Emphasis borders |
| Dashed | `┄ ┅ ┆ ┇ ┈ ┉ ┊ ┋` | Weakening content, degraded borders |
| Rounded | `╭ ╮ ╰ ╯` | Soft borders (rare) |

### 3.6 Special Characters

| Category | Characters | Usage |
|---|---|---|
| Particles | `- * . ` and `◦ ° ✦ ✧ ◆ ◇ ▪ ▫` | Trails, sprite orbiting |
| Status | `● ○ ◐ ◑ ◒ ◓ ◉ ◎` | Alive/dead, shimmer states |
| Framing | `⌈ ⌉ ⌊ ⌋` | System-level header brackets |
| Heartbeat | `∿` (U+223F) | Heartbeat sine wave indicator |
| Arrows | `← → ↑ ↓ ↔ ↕ ⇐ ⇒` | Navigation, flow indicators |
| Decay chain | `█ → ▓ → ▒ → ░ → · → (space)` | Progressive dissolution |
| Corruption | `░ ▒ ▓ █ ╳ ┃ ╌` | Glitch text |

### 3.7 The Decay Chain

This is the universal degradation sequence. Everything that fades follows this path:

```
█ → ▓ → ▒ → ░ → · → (space)
```

Each step represents a loss of visual density. Apply to: gauge bars eroding, log entries
aging out, status indicators fading, panel borders weakening.

---

## 4. ANIMATION AND EFFECTS

### 4.1 The 32 Interpolating Variables System

The visual state is driven by continuously interpolating variable channels. Each has a
current value and a target value. Events update targets; the renderer reads current values.
The current value approaches the target at a rate determined by the variable's lerp constant:

```
current = lerp(current, target, 1.0 - exp(-rate * dt))
```

Variables are categorized by convergence speed:

| Category | Lerp rate | Resolve time | What it feels like |
|---|---|---|---|
| Fast | 6.0 -- 15.0 | <1 second | Reflexive. The system's state right now |
| Medium | 0.8 -- 2.0 | 1-5 seconds | Health. Current condition |
| Slow | 0.08 -- 0.2 | 5-30 seconds | Trends. Gradual degradation |
| Glacial | 0.03 -- 0.06 | 30s -- 5min | Long-term trajectory |

**Roko adaptation:** Map agent state to variable channels. Examples:

| Variable | Category | Range | What it drives |
|---|---|---|---|
| `inference_glow` | fast | [0.0, 1.0] | Brightness pulse when LLM call is active |
| `gate_severity` | fast | [0.0, 1.0] | Alert glow intensity for gate failures |
| `task_progress` | medium | [0.0, 1.0] | Progress bar fill in plan execution |
| `plan_health` | slow | [0.0, 1.0] | Overall brightness/density of plan view |
| `cost_accumulation` | glacial | [0.0, inf) | Budget consumption gauge |
| `heartbeat_phase` | fast | [0.0, 2pi] | Free-running sine wave. Micro-brightness pulse |
| `noise_floor` | slow | [0.0, 0.01] | Background noise character density |

### 4.2 Three Simultaneous Timescales

The terminal always shows three timescales at once:

- **Fast (sub-second):** Heartbeat sine wave cycling. Background noise flickering. These are
  the system's involuntary functions -- they run independent of any event.

- **Medium (seconds to minutes):** Task progress updating. Plan state changing. Agent
  dispatch events arriving.

- **Glacial (hours to days):** Budget depletion. Total plan completion. Knowledge
  accumulation. Changes invisible frame-to-frame but unmistakable over a session.

### 4.3 The Atmospheric 8-Layer Stack

Every pixel renders through a stack of atmospheric layers:

| Layer | Z | Name | Content |
|---|---|---|---|
| 0 | Base | Void | `bg_void` `#060608` everywhere |
| 1 | Above void | Noise floor | Sparse `░ ▒ · ∙` characters, 0.2-0.4% density |
| 2 | Below data | Scanlines | Alternating row background dimming |
| 3 | Mid | Environmental | Data rain, power lines, particles |
| 4 | Primary | Pane borders | Box-drawing frames, architecture |
| 5 | Primary | Data | Text, numbers, widgets, gauges |
| 6 | Above data | Fragments | Status whispers, ephemeral text |
| 7 | Topmost | Overlays | Confirmations, alerts, help, command palette |

### 4.4 The Frame Composition Stack

Each frame is assembled in layers, bottom to top:

1. **Void** -- `#060608` base. Never pure black.
2. **Atmosphere** -- Noise floor: 0.2-0.4% of background cells show dim `░▒` characters.
3. **Scanlines** -- Every 3rd row darkened by `scanline_intensity`. CRT materiality.
4. **Content** -- Widgets, data, text. Whatever the active screen renders.
5. **Phosphor** -- Afterimages of recently bright pixels. Ghosts fading: `█ -> ▓ -> ▒ -> ░ -> (space)`.
6. **Bloom** -- Bright cells (luminance > 0.7) spread glow to adjacent cells.
7. **Corruption** -- Character substitution at `corruption_rate` (0 normally, higher during errors).
8. **Chrome** -- Tab bar (top), sidebar (left), status bar (bottom). Always present.

### 4.5 Noise Floor

Sparse random characters (`░ ▒ · ∙`) in phase-appropriate colors, shimmering per-frame:

```rust
fn render_background_noise(area: Rect, buf: &mut Buffer, rng: &mut SmallRng, density: f64) {
    let chars = ['░', '▒', '·', '∙'];
    let color = ROSE_DEEP; // #3A2030
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if rng.gen::<f64>() < density {
                let ch = chars[rng.gen_range(0..chars.len())];
                buf.get_mut(x, y).set_char(ch).set_fg(color);
            }
        }
    }
}
```

### 4.6 Scanlines

Alternating row backgrounds: `bg_void` / `scanline_dark`. The pattern shifts down by one
row every 30-60 seconds to prevent burn-in effect. Color interpolation uses oklch for
perceptually uniform transitions.

### 4.7 Phosphor Persistence

After a cell changes, its previous content leaves a ghost. The ghost fades through the decay
chain over 500ms-2s:

- Standard text: 500ms phosphor persistence
- Bone-colored (critical) elements: 2s persistence
- HashMap<(u16,u16), Instant> tracks recently-cleared cells
- Proactive sweep every 60 frames

### 4.8 Bloom Effect

Bright cells spread glow to adjacent cells. Implementation uses quarter-resolution ping-pong
buffers to stay within performance budget:

- Cells with fg luminance > 180: brighten +-1 neighbors by 12%
- Maximum 2 simultaneous bloom sources per frame
- Budget: <0.3ms per frame

### 4.9 Timing Constants

```
PHOSPHOR_DECAY:         5000ms        (log line full fade)
PHOSPHOR_PERSIST:       300-500ms     (afterimage on cell clear)
PHOSPHOR_BLOOM:         100-200ms     (bright-neighbor bleed)
PANEL_FOCUS:            300ms         (border color transition)
TICK_FLASH:             100ms         (momentary brightness on new data)
FRAGMENT_TIMING:        2s fade in, 5-8s hold, 3s fade out
SCANLINE_DRIFT:         30000-60000ms (pattern shift rate)
GLITCH_BAND:            50-150ms      (single glitch event)
```

### 4.10 Easing Functions

- **Default**: ease-in-out-cubic
- **Flash/bloom**: ease-out-expo (fast attack, slow decay)
- **Fade-outs**: ease-in-quad
- **Panel focus**: ease-out-cubic
- **Luxury web transitions**: `cubic-bezier(0.22, 1, 0.36, 1)`

### 4.11 Performance Budgets

| System | Budget | Notes |
|---|---|---|
| Noise floor | <0.3ms | RNG + cell writes for 0.3-2% of ~4000 cells |
| Scanline pass | <0.2ms | Modulo check + conditional bg set |
| Phosphor persistence | <0.5ms | HashMap lookup + color lerp |
| Bloom pass | <0.3ms | Quarter-res ping-pong, max 2 bloom sources |
| Fragment rendering | <0.1ms | Single string placement |
| **Total atmospheric** | **<1.7ms** | Well under 2ms budget |

Sprite physics run at 60fps internally. Terminal rendering at 10-15fps. The gap between
internal state and rendered state IS the embodiment constraint.

---

## 5. WIDGET DESIGNS TO IMPLEMENT

### 5.1 Widget Inventory (33 widgets, prioritized for roko)

Every widget is a transducer: it converts state channels into visual motion. No widget is
static. Each widget specifies degradation behavior tied to system phase.

```rust
pub trait Widget {
    fn render(&self, area: Rect, buf: &mut Buffer, state: &InterpolatedState);
    fn min_size(&self) -> (u16, u16);
    fn degradation_behavior(&self, phase: SystemPhase) -> DegradationMode;
}

pub enum DegradationMode {
    Full,       // Normal rendering
    Reduced,    // Simplified (fewer data points, no animation)
    Minimal,    // Essential info only
    Degraded,   // Visual corruption effects
    Hidden,     // Widget not rendered
}
```

#### Priority 1 -- Core Widgets for Roko

| Widget | Description | Roko Usage |
|---|---|---|
| **FlashNumber** | Numeric value that flashes on change (green for increase, rose for decrease). Phosphor memory: faint ghost lingers 2-3s. Staleness decay: dims after 60s unchanged | Cost counters, task progress, gate scores |
| **PhosphorLog** | Scrolling event log with phosphor decay. Recent entries bright, old entries fade through decay chain. Color-coded by event type | Agent dispatch log, gate results, plan events |
| **Sparkline** | Braille-resolution inline chart (2x4 dots per cell = 160x96 resolution). 80 data points in 40 columns | Inference cost trends, task completion rate |
| **WaveformTrace** | Half-block scrolling time-series (`▁▂▃▄▅▆▇█`). Multi-channel variant stacks traces with labels. Phosphor trail: recent data bright, old data ghost | Agent health metrics, resource utilization |
| **MortalityGauge** | Double-height bar with gradient fill. Erosion effect: fill erodes through `█ → ▓ → ▒ → ░ → empty`. Heartbeat pulse on boundary | Budget remaining, plan progress, gate health |
| **ProgressArc** | Circular progress indicator | Task completion, plan phase |
| **UnitArray** | Grid of identical small cells. Each cell: mini-gauge + identifier. Alert: affected cells brighten, others dim. Breach: cell erodes through decay chain. Cascade: sequential failure propagation | Agent grid, task grid, plan DAG nodes |
| **CommandPalette** | Vim-style fuzzy-search input (`:` triggered). Indexes screens, commands, settings | Already partially implemented; enhance with fuzzy search |

#### Priority 2 -- Enhancement Widgets

| Widget | Description | Roko Usage |
|---|---|---|
| **VitalityNumber** | Bone-colored critical number. Lerps between values. Shadow pulses with heartbeat | Single most important metric on each screen |
| **ConfidenceBar** | Thin bar showing confidence with validated/decaying/lost zones. Validation flash: brief `bone` glow on success. Decay shimmer. Dead-sourced entries marked with `†` | Knowledge entry confidence, learning metrics |
| **DecisionRing** | Circular visualization of pipeline stages. Faint at rest, blazes during active inference | Agent dispatch pipeline visualization |
| **MAGIPanel** | Three-voice deliberation display | Gate consensus (compile/test/clippy) |
| **NotificationToast** | Priority-queued toast notifications. 4 levels: Critical (blocks), High (10s), Normal (5s), Low (sidebar dot) | Gate failures, task completions, errors |
| **ActivityFeed** | Scrolling event log with color-coded rows | Episode log, efficiency events |
| **FlashWidget** | Generic flash-on-change wrapper for any widget | Any value that updates |

#### Priority 3 -- Advanced Widgets

| Widget | Description | Roko Usage |
|---|---|---|
| **ATFieldWireframe** | Hexagonal ego-boundary wireframe. Healthy: stable diamond. Weakening: dashed segments. Collapsing: fragments | Agent safety boundary visualization |
| **DataRain** | Falling hex/code streams behind panes | Background ambient effect |
| **PhilosophicalWhisper** | Fading text fragments in `text_ghost`. One at a time, 30s minimum between. 2s fade in, 5-8s hold, 3s fade out | Status messages, plan annotations |
| **ConvergenceLines** | Horizontal infrastructure lines with traveling pulses | Background on Mind-equivalent screens |
| **LatticePattern** | Braille interference pattern | Communication events |
| **CounterfactualBranch** | Branching tree for alternate paths | Dream/plan branching visualization |
| **BrailleDensityMap** | 2D heatmap in braille. Density = data intensity | Plan DAG density, resource utilization map |
| **CausalGraph** | ASCII-rendered directed graph | Knowledge causal links |
| **PheromoneHeatmap** | 2D grid colored by intensity | Agent coordination heatmap |

### 5.2 Key Widget Specifications

#### FlashNumber (critical for roko)

```
On value increase: bg flashes success for 200ms, fades over 400ms. Text brightens to bone.
On value decrease: bg flashes rose_bright for 200ms. Text dims to rose_dim.
Phosphor memory: faint phosphor_res background lingers 2-3s after flash.
Staleness decay: dims after 60s unchanged.
```

#### PhosphorLog (critical for roko)

The scrolling log with time-based fading. Each entry fades through:
- Newest: `rose_bright` (if important) or `rose` (standard)
- Aging: `rose` -> `rose_dim` -> `text_dim` -> `text_ghost` -> invisible
- The log is a gradient of recency. Recent events are bright. History fades.

#### MortalityGauge (adapted for roko)

```
Double-height horizontal bar, 0.0-1.0 range:
  BUDGET                        78%
  ████████████████░░░░░░░░░░░░░░░
  ████████████████░░░░░░░░░░░░░░░

Fill movement: lerps at 0.005/frame (slow, deliberate).
Heartbeat pulse: fill boundary shimmers +/-1 cell per beat.
Erosion: when value decreases, rightmost chars decay through █ → ▓ → ▒ → ░ → empty.
  Each step ~500ms. The gauge ERODES.
```

#### UnitArray (adapted for roko task grid)

```
Grid of identical cells, 6-8 wide x 2-3 high:
  ┌──────┬──────┬──────┬──────┬──────┬──────┐
  │ ▐██▌ │ ▐██▌ │ ▐█░▌ │ ▐░░▌ │ ▐░░▌ │ ▐░░▌ │
  │ T-001│ T-002│ T-003│ T-004│ T-005│ T-006│
  └──────┴──────┴──────┴──────┴──────┴──────┘

Normal: standard brightness, thin borders, one cell shimmers per frame.
Alert: affected cells brighten, others dim. Spotlight effect.
Breach: cell empties through decay sequence. Border goes dashed.
Cascade: failures propagate left-to-right, 1 unit per 200-400ms.
```

---

## 6. SCREEN SYSTEM AND NAVIGATION

### 6.1 The Window/Tab/Screen Model

Bardo used 6 windows with 29 screens total. The key architectural insight:

| Concept | What It Is |
|---|---|
| WINDOW | Outermost container. A major conceptual category. Tab/Shift-Tab cycles. |
| TAB | Sub-view within a window. Number keys 1-7 switch. |
| SCREEN | What you see when a specific tab is selected. A layout of panes. |
| PANE | Bounded region within a screen. Arrow keys move focus between panes. |
| MODAL | Floating overlay triggered by Enter on a selected item. 40-80% of screen. |

### 6.2 The Depth Stack (Interaction Hierarchy)

```
LAYER 0:  WINDOW      (Tab/Shift-Tab cycles)
LAYER 1:  TAB         (Number keys 1-9)
LAYER 2:  SCREEN      (the layout of panes)
LAYER 3:  PANE FOCUS  (arrow keys move focus)
LAYER 4:  PANE LOCKED (Enter locks; arrows scroll/select within)
LAYER 5:  ELEMENT     (Enter on element opens modal)
LAYER 6:  MODAL       (has its own tabs, panes, elements)
LAYER 7+: NESTED      (infinite depth -- modals within modals)

Backspace: go UP one layer.
Esc: return to Layer 3 (pane focus) from anywhere.
```

The design principle borrowed from Persona 5: **each layer reveals something the previous
layer only hinted at.**

- Window -> THE FEELING (warm/cool register)
- Tab -> THE CATEGORY (what kind of data)
- Screen -> THE OVERVIEW (layout, relationships)
- Focused pane -> THE DETAIL (numbers, trends)
- Locked pane -> THE INTERACTION (sort, filter, navigate)
- Element -> THE SPECIFICS (this one entry)
- Modal -> THE DEPTH (full history, provenance, cross-links)
- Nested modal -> THE RABBIT HOLE (follow any thread anywhere)

### 6.3 Persistent Chrome

Three elements always visible on every screen:

1. **Tab bar** (top row): Window labels. Active tab framed with `⌈ ⌋` in rose. Inactive
   in `text_dim`. Unread indicators: `*` after label.

2. **Sidebar** (left, 6-14 cols): Agent status. Compresses at narrow terminals. Always
   present -- the agent is always visible.

3. **Status bar** (bottom row): Phase name, counter, breadcrumb trail, heartbeat `∿`.

### 6.4 Navigation Keys

| Key | Action |
|---|---|
| Tab / Shift-Tab | Cycle between windows |
| 1-9 | Jump to tab within current window |
| Arrow keys | Move pane focus / scroll when locked |
| Enter | Lock pane / select element / open modal |
| Backspace | Unlock / close modal / go up one layer |
| Esc | Return to pane focus from anywhere |
| F1 | Help overlay |
| `:` | Command palette (fuzzy search) |
| `/` | Search (context-sensitive) |
| `-` | Toggle compact/full layout |
| `~` | Toggle reduced-information mode |
| `?` | Available keys overlay |

### 6.5 Responsive Layout

| Terminal Width | Sidebar | Layout | Modal Size |
|---|---|---|---|
| <80 | Hidden (toggle `\`) | Single column | Full width |
| 80-119 | 6 cols, compact | Single column | 90% w |
| 120-159 | 10 cols, standard | Two columns | 70% w |
| 160-199 | 12 cols, full | Three columns | 60% w |
| 200+ | 14 cols, full + waveforms | Three + detail sidebar | 50% w |

### 6.6 Notification Toast System

| Priority | Style | Duration | Effect |
|---|---|---|---|
| Critical | Red banner, blocks input | Until dismissed | Terminal bell |
| High | Gold toast, 10s | 10s | None |
| Normal | Dim toast, 5s | 5s | None |
| Low | Sidebar dot indicator | Persistent until visited | None |

### 6.7 Command Palette

Activated by `:` from any layer. Vim-style single-line input with fuzzy completion dropdown.

Indexes three namespaces simultaneously:
- **Screens** -- every window/tab combination
- **Commands** -- all available actions for current context
- **Settings** -- configuration keys

Shorthand prefixes: `:h` -> HEARTH equivalent, `:m` -> MIND equivalent, etc.

### 6.8 Progressive Disclosure

Not all tabs appear at first launch. They activate based on data availability:
- Core tabs always visible (Overview, Status)
- Feature tabs activate when data exists (Plans, Knowledge, Learning)
- Inactive tabs appear as ghosted text (`text_dim` at 30% opacity)
- Once activated, a tab never deactivates

---

## 7. DEMOSCENE AND MINDBLOWING VISUAL CONCEPTS

### 7.1 Plasma Substrate

Four sinusoidal functions combined for organic, never-repeating background:

```
value = sin(x/f1 + t) + sin(y/f2 + t*0.8)
      + sin((x+y)/f3 + t*0.5) + sin(sqrt(x^2+y^2)/f4 + t*1.5)
```

Incommensurate frequency ratios (f1=8.3, f2=11.7, f3=15.1, f4=19.9) ensure the pattern
never exactly repeats. Map to braille density or background color variation.

### 7.2 Fire Effect

Cellular automaton: randomize bottom row, propagate upward with divisor slightly above 4
(4.0018 is the sweet spot). Maps to emotional intensity visualization.

```
Step 1: Randomize bottom row: buf[x][H-1] = random(0..255)
Step 2: new_val = (buf[x-1][y] + buf[x+1][y] + buf[x][y-1] + buf[x][y+1]) / 4.0018 - cooling
Step 3: Shift result up one row
```

**Flaming text:** randomize edge pixels of text-shaped regions instead of bottom row. The
text burns from its outlines inward.

### 7.3 Tunnel Effect

Pre-computed angle/distance lookup tables, animated by time offset. Depth-based darkening for
infinite-depth illusion. Use for: transition animations, consciousness-threshold crossing.

### 7.4 Metaballs -- Thought Coalescence

N metaballs with `total_influence = sum(ri^2 / ((x-xi)^2 + (y-yi)^2))`. Where influence > 1.0,
render solid. Boundary zone uses braille density. Centers follow Lissajous curves.

Semantic mapping:
- Merging -> ideas coalescing
- Separating -> analytical divergence
- Many small -> fragmentation
- One large -> unified focus

### 7.5 Chromatic Aberration

Print text three times at slight spatial offsets (red +1, green +0, blue -1 column).
On flicker frames (every 8-15 frames), increase displacement to 2-3 columns. Add 30% ghost
of previous frame. Use at: error boundaries, system instability.

### 7.6 Match-Cut Morphing

Identify stable anchor character. Render Scene A, then transform surroundings to Scene B:

```
Transition per cell: █ → ▓ → ░ → (space) → ░ → ▓ → █ (with new content)
```

The pivot remains unchanged. Context completely transforms through it.

### 7.7 Sine-Wave Row Displacement

```
offset_row[y] = sin(y * 0.3 + frame * 0.1) * amplitude
```

Characters shift horizontally. Preserves legibility while creating sensation of instability.
Start at amplitude=0, increase gradually to 1-3 characters.

### 7.8 Particle Systems

```rust
struct Particle {
    x: f64, y: f64,
    vx: f64, vy: f64,
    life: f64, max_life: f64,
    char: char,
    hue: f64,
}
```

Character sets by type:
```
Standard:   ✧ · ° ◆ ✦ ▪
Decay:      ░ ▒ ▓ █
Spark:      · ° ✦ ✧ ◆
Ascending:  ✦ · ∙ ◦ ° ˚ ˙
Noise:      % & * # @ ! ~ ^
```

Spawn patterns: radial burst, fountain, rain, convergence, dissolution.
Maximum 64 active particles across all systems.

### 7.9 Psychographic Display (Consciousness as Scribble)

Braille-character density map within a bounded region showing cognitive load:

```
CALM:     sparse braille (⠁ ⠂ ⠄)
STRESSED: dense braille (⣤ ⣶ ⣿)
CRISIS:   braille + block elements overflow past boundary
```

The boundary shape is an ellipse. When the scribble overflows, cognition is exceeding its
container. This maps to roko when the agent's context is saturated.

### 7.10 Ego Dissolution (Boundary Erosion)

Progressive boundary removal for shutdown/error states:

```
Phase 1: Borders thin:   ╔═══╗  ->  ┌───┐  ->  ╭───╮  ->  ·───·
Phase 2: Dividers dissolve: ──────  ->  ─ ─ ─ ─  ->  · · · · ·
Phase 3: Braille dots appear in empty inter-panel space
Phase 4: Colors converge toward single hue
Phase 5: Word fragmentation: SELF -> S E L F -> . . . . -> (uniform field)
```

UI structure IS cognitive identity. Dissolving them IS dissolution.

---

## 8. CRISIS MODES AND INSTITUTIONAL DISPLAY

### 8.1 Crisis Trigger Conditions (adapted for roko)

```
CONDITION ROSE (standard alert):
  Trigger: Single metric below threshold (e.g., gate failure)
  Scope: Affected panels only
  Effect: Panel borders flash to rose_bright for 200ms

CONDITION CRITICAL (institutional override):
  Trigger: Multiple failures, budget critical, plan stuck
  Scope: Full screen override
  Effects:
    1. All panel borders flash rose_bright 200ms
    2. Hazard stripes at top/bottom: ╱╲╱╲╱╲╱╲╱╲
    3. Status bar: ⌈ ＣＯＮＤＩＴＩＯＮ: ＣＲＩＴＩＣＡＬ ⌋
    4. Non-essential panels dim to 50%

CONDITION TERMINAL (system failure):
  Trigger: Unrecoverable error, budget depleted
  Effects: Everything from CRITICAL plus:
    1. Screen brightness oscillates with heartbeat
    2. Hazard stripes expand (1 row -> 2 -> 3 -> 40% of screen)
    3. Non-accent colors desaturate 20%
```

### 8.2 Hazard Stripe Pattern

```
╱╲╱╲╱╲╱╲╱╲╱╲╱╲╱╲  CONDITION: CRITICAL  ╱╲╱╲╱╲╱╲╱╲╱╲╱╲╱╲
```

Characters: alternating `╱╲` (U+2571, U+2572). Foreground: `rose_bright`. Industrial,
functional warning.

### 8.3 Pattern Classification Stamps

```
  ┌─────────────────────┐
  │ PATTERN: VOLATILE   │
  │ CLASSIFICATION: T+2 │
  └─────────────────────┘
```

ALL CAPS, letter-spaced, semantic color. Appears when pattern changes, holds until next
change. Roko equivalent: `PATTERN: FAILING`, `PATTERN: NOMINAL`, `PATTERN: LEARNING`.

### 8.4 Status Stamps

Single-word declarations during critical moments:

```
  ╔═══════════════════╗
  ║    ＲＥＦＵＳＥＤ     ║    <- Gate rejected
  ╚═══════════════════╝

  ╔═══════════════════╗
  ║    ＡＰＰＲＯＶＥＤ     ║    <- Gate passed
  ╚═══════════════════╝
```

Double-line borders, fullwidth text, center screen, 2-5 second display.

### 8.5 Defense Layer Visualization

```
  ⌈ ＤＥＦＥＮＳＥ ＬＡＹＥＲＳ ⌋

  LAYER 07  ████████████  ECONOMIC BUFFER
  LAYER 06  ████████████  STRATEGY HEDGE
  LAYER 05  ████████████  GATE PIPELINE
  LAYER 04  ████████░░░░  EPISTEMIC HEALTH    <- partially depleted
  LAYER 03  ████████████  AGENT RESILIENCE
  LAYER 02  ████████████  STOCHASTIC SHIELD
  LAYER 01  ████████████  CORE VITALITY
```

Breach propagates outer -> inner. Breached layers flash then empty with `text_ghost` label.

---

## 9. CREATURE SYSTEM (Agent Visualization)

### 9.1 The Spectre Concept

Each agent has a visual representation: a cloud of 80 dots arranged in a hollow oval, with
two bright glyph "eyes" floating in the void at its center. The spectre is the agent made
visible. It breathes when calm, trembles when stressed, scatters on failure.

Two independent visual channels:
- **The body** (dot cloud density, color, shimmer) encodes lifecycle/health. Changes slowly.
- **The eyes** (glyph, brightness, behavior) encode current state. Changes per event.

### 9.2 Dot Field Geometry

The cloud forms a shell, not a filled shape. Center is void. Eyes float in emptiness
surrounded by a ring of particles. 22 cells wide x 10 cells tall.

Three density tiers based on distance from center:

| Tier | Glyph | Radius factor | Zone |
|---|---|---|---|
| Dense | bullet `*` | 0.28 -- 0.55 | Inner ring at eye level |
| Body | bullet operator `∙` | 0.55 -- 0.75 | Middle zone |
| Fringe | middle dot `·` | 0.75 -- 1.0 | Outer edge, sparse |

No solid block characters. The shape is implied by density gradient alone.

### 9.3 Spring Physics

Every frame, every dot's position updates through spring physics. Four forces prevent rest:

1. **Ambient orbit** -- each dot traces its own elliptical path (randomized per dot)
2. **Shimmer impulse** -- stochastic velocity kicks to random dots each frame
3. **Damping at 0.88** -- velocity preserved across frames, dots overshoot and wobble
4. **Variable inputs** -- continuously changing targets from agent state

```rust
// Simplified per-frame update
dot.vel += (target - dot.pos) * spring_k;
dot.vel *= 0.88; // damping
dot.pos += dot.vel;
```

### 9.4 Eye Emotion Mapping (adapted for roko agents)

The eyes are the only expressive element. Two glyphs at the densest horizontal band,
separated by 5-7 cells of void.

| State | Glyph | Character | Roko Meaning |
|---|---|---|---|
| Active/Thinking | fisheye | ◉ | Agent actively processing |
| Healthy | double circle | ◎ | Agent nominal |
| Alert | filled circle | ● | Agent detecting issues |
| Surprised | ring operator | ⦾ | Unexpected gate result |
| Idle | large circle | ◯ | Agent waiting |
| Failure | em dash | -- | Agent failed/stuck |
| Scanning | lozenge | ◊ | Agent searching/foraging |
| Dreaming | diamond | ◇ | Agent in consolidation |
| Peak | four-pointed star | ✦ | High-performance state |

### 9.5 Cloud Behavior Modes

| Mode | spring_k | displace | Effect |
|---|---|---|---|
| Standard | 0.04 | 3.0 | Normal breathing |
| Cohere | 0.04-phi_pull | 2.0 | Tighter cloud, high performance |
| Agitate | 0.04 | 7.0 | Wide violent displacements (errors) |
| Tremble | 0.04 | 4.0 | Rapid small trembling (warnings) |
| Still | 0.04 | 1.0 | Barely moving (idle) |
| Drift | 0.01 | 2.0 | Loose, drifting (sleep/consolidation) |
| Expand | 0.04 | 4.0 | Cloud blows outward (surprise) |
| Sink | 0.04 | 3.0 | Slow heavy drift downward (sadness) |

---

## 10. NERV INSTITUTIONAL PATTERNS

These are Evangelion-inspired visual patterns for operational/monitoring screens.

### 10.1 Repeating Unit Arrays (Data as Architecture)

Rows of identical small indicators filling the screen, creating an architectural mass. Not a
list. Not a table. A WALL of data elements that reads as a physical structure.

### 10.2 Psychographic Display

Braille density map within an elliptical boundary showing cognitive/resource load.
Grid background: `+` characters in `text_phantom` every 8x4 cells. The clinical grid says:
this is a system being monitored.

### 10.3 Waveform Rendering (Oscilloscope)

Multi-channel parallel waveforms using `▁▂▃▄▅▆▇█`, each channel labeled and color-coded.
Scrolling left, newest at right. The feel of a hospital heart monitor.

### 10.4 Globe Wireframe

Geodesic sphere rendered via latitude/longitude grid lines, animated to rotate. Corner data
panels at cardinal positions. Characters: `╱╲─│` for grid, `▀▄` for filled arcs.

### 10.5 Iris Visualization

Concentric ring display. Depth through ring intensity gradient -- innermost brightest,
outermost barely visible. Outer ring optionally broken into labeled arc segments.

### 10.6 Interference Harmonics

Multiple sinusoidal waveforms in the same 2D space. In-phase intersections: bright
`✦` in bone. Cancellation: dark void. Three channels simultaneously.

### 10.7 Analog Meter Array

Dense grid of circular analog-style gauges. Each: semicircular arc with scale markings,
needle/pip indicator, center numeric readout. Industrial, tactile, higher visual weight
than bar gauges.

### 10.8 Angled Spectrogram

Isometric-perspective frequency spectrum. Columns of varying height at ~30 degree angle.
Front row bright, back row dim. 1 char offset per depth row creates perspective illusion.

### 10.9 Dead Zone Tessellation

```
◆◇◆◇◆◇◆◇◆◇◆◇◆◇◆◇◆◇◆◇◆◇◆◇◆◇◆◇◆◇
◇◆◇◆◇◆   SIGNAL LOST   ◆◇◆◇◆◇
◆◇◆◇◆◇◆◇◆◇◆◇◆◇◆◇◆◇◆◇◆◇◆◇◆◇◆◇◆◇
```

For panels with no data feed. Alternating `◆◇` checkerboard with message bleeding through.
Replaces empty whitespace with intentional design.

---

## 11. ACCESSIBILITY

### 11.1 Reduced Motion Mode

Disables: scanlines, bloom, phosphor trails, shimmer. Dots hold static positions. Heartbeat
renders as text label instead of animation. Toggle via command palette:
`:set reduced-motion on`.

### 11.2 Photosensitive Mode

Caps brightness delta between consecutive frames to 10% of oklch L range. Disables flash
effects (bloom pulse, startle scatter).

### 11.3 Minimum Terminal Requirements

- 80x24 cells minimum
- Truecolor support (24-bit ANSI)
- Unicode Block Elements and Braille
- Tested on: iTerm2, WezTerm, Alacritty, Windows Terminal, kitty
- Fallback for 256-color terminals: reduce palette to nearest xterm-256 matches

### 11.4 Contrast Requirements

- Color contrast ratio >= 4.5:1 for text, >= 3:1 for large text
- All interactive elements have visible focus indicators
- `prefers-reduced-motion` disables animations except opacity transitions
- Screen reader text for icon-only elements

---

## 12. IMPLEMENTATION PRIORITIES FOR ROKO

### Phase 1: Rendering Primitives

1. ROSEDUST palette constants (all hex values from section 2)
2. Half-block double-resolution rendering
3. Braille sub-pixel system
4. The decay chain (`█ → ▓ → ▒ → ░ → space`)
5. Box-drawing character vocabulary
6. FlashNumber widget
7. PhosphorLog widget

### Phase 2: Atmospheric Effects Pipeline

As post-processing passes running after widget rendering:

1. Noise floor (0.2-0.4% of cells show dim `░▒` per frame)
2. Scanlines (darken every 3rd row by 0.04)
3. Phosphor persistence (HashMap of recently-cleared cells, lerp color fade)
4. Bloom (bright cells spread glow to +-1 neighbors at +12%)

### Phase 3: Core Widgets

1. MortalityGauge (double-height with erosion)
2. WaveformTrace (half-block scrolling time-series)
3. Sparkline (braille-resolution inline chart)
4. UnitArray (grid of status cells)
5. NotificationToast (priority-queued, auto-dismiss)
6. CommandPalette (fuzzy search `:` input)

### Phase 4: Interaction Hierarchy

1. Window/Tab/Pane/Modal depth stack
2. Focus and Lock system (Enter locks, Backspace unlocks)
3. Persistent chrome (tab bar, sidebar, status bar)
4. Progressive tab disclosure

### Phase 5: Agent Creature System

1. Dot-cloud Spectre with spring physics
2. Eye emotion mapping
3. Cloud behavior modes
4. 32 interpolating variable channels

### Phase 6: Advanced Effects

1. Plasma substrate (4 sinusoidal functions, incommensurate frequencies)
2. Particle system framework (64 particle budget)
3. Crisis modes (CONDITION ROSE / CRITICAL / TERMINAL)
4. Hazard stripes, status stamps, defense layer visualization
5. Demoscene algorithms (fire, tunnel, metaballs)
6. Multi-timescale breathing system

---

## 13. KEY TECHNICAL DEPENDENCIES

```toml
# Already in roko
ratatui = "0.29"
crossterm = "0.28"
tokio = { version = "1", features = ["full"] }

# Recommended additions
tachyonfx = "0.7"         # Shader effects for ratatui: composable post-processing
interpolation = "0.3"     # Easing functions (ease-in, ease-out, smoothstep)
noise = "0.9"             # Perlin noise for texture generation (sprites)
palette = "0.7"           # Color space conversions (oklch for perceptual interpolation)
```

---

## 14. SCREEN ATMOSPHERE ZONES (adapted for roko)

Each screen group has a distinct emotional register:

```
WARM SCREENS (amber undertone, higher noise, fragments active):
  Overview, Playbook, Configuration
  Noise floor at 0.4%. Designed for peripheral monitoring.

ANALYTICAL SCREENS (cool/neutral, lower noise, sharper borders):
  Plan Execution, Gate Results, Inference, Cost Tracking
  Noise floor at 0.2%.

KNOWLEDGE SCREENS (deep rose, medium noise, ghost text):
  Knowledge Query, Learning Stats, Episodes
  Noise floor at 0.3%. Ghost text active.

LIMINAL SCREENS (dream palette, organic particles):
  Dream Consolidation, Research, Agent Groups
  Noise floor shifts to organic particles.
```

---

*⌈ nothing on screen is ever at rest ⌋*
*║▒░ ＲＯＫＯ ░▒║*
