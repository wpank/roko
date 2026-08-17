# 10. Expressive Primitives — Advanced Component Library

Extends `09-DESIGN-PRIMITIVES.md` with higher-order components: resizable panes, loading transitions, stepped progress variants, WebGL backgrounds, agent-namespaced widgets, event feeds, floating chat, modals/overlays, and robust layout primitives.

These are **wave 2-4** components. They build on the foundation primitives (Panel, Surface, Skeleton, Badge, etc.) and assume the token system, animation system, and lib utilities from `09` are in place.

---

## 1. Design Philosophy

**Expressive, not decorative.** Every primitive in this doc exists because the demo needs to communicate state that the foundation primitives cannot express alone:

- **Resizable panes** -- the demo is a multi-panel control plane; fixed layouts break at different viewport sizes and use cases
- **Loading transitions** -- the system has real latency (LLM inference, gate pipelines); transitions must feel intentional, not broken
- **Progress variants** -- the orchestration loop has 5-15 steps with branching; a simple progress bar cannot represent this
- **WebGL backgrounds** -- ambient state (agent affect, system health, lifecycle phase) needs always-on expression without competing with content
- **Agent-namespaced components** -- the multi-agent model is Roko's differentiator; agent identity must be visible everywhere
- **Event feeds** -- real-time events are the primary data type; they need dedicated rendering

**Governing layout principles** (cross-reference `04-DESIGN-SYSTEM.md` sections 8-10, `02-ARCHITECTURE.md` section 6):

- **Scrollable density-first layout.** All expressive primitives render within a scrollable page container. No component in this doc assumes a fixed viewport or `height: 100vh`. Panes, agent containers, and event feeds live inside the scrollable flow, not pinned to viewport edges.
- **Terminal aesthetic extends to ALL new components.** Every component here uses the same mono chrome / ASCII vocabulary established in `09-DESIGN-PRIMITIVES.md` section 11 and `04-DESIGN-SYSTEM.md` section 9. Labels are mono uppercase. Dividers use box-drawing characters. Status indicators use ASCII glyphs.
- **Agent containers are scrollable cards within the page flow**, not fixed full-screen panels. An `<AgentContainer>` renders as a dense card that participates in the normal page scroll, not a viewport-locked panel. Internal scroll for feeds/logs uses `max-height` with fade gradients.
- **Event feeds are compact scrollable sections**, not dedicated fixed panels. `<EventStream>`, `<AgentFeed>`, and `<InferenceFeed>` all use `max-height` with internal `overflow-y: auto` and gradient fade at edges. They sit within the page flow alongside other content.

**Performance budget:** WebGL backgrounds run at 30fps max, deferring to content rendering. Canvas/WebGL components check `prefers-reduced-motion` and degrade to CSS gradients. No component in this doc may cause layout thrashing on the main thread.

**Dependency rule:** Every component here imports from `primitives/` (09's catalog). None reimplements what already exists there.

---

## 2. Resizable Pane System

Replaces the basic `<Split>` from 09 with a full pane management system modeled on VS Code's workbench layout.

**Layout context:** Pane grids and groups live INSIDE the scrollable page container, not viewport-locked. A `<PaneGrid>` or `<PaneGroup>` fills its parent container (which may be a section of the page), not the viewport. When a page has a pane layout, the pane container itself has a content-determined height (or a reasonable `max-height`), and the page remains scrollable above and below it. This is distinct from VS Code where panes fill `100vh` -- here, they are one section among many in a scrollable page.

### 2.1 `<ResizablePane>` — Resizable Container

A single pane that can be resized by dragging its edges. Multiple panes tile together within a `<PaneGrid>` or `<PaneGroup>`.

```tsx
interface ResizablePaneProps {
  // Identity
  id: string;                    // Unique within parent PaneGrid/PaneGroup
  label?: string;                // Mono uppercase header label
  icon?: ReactNode;              // Header icon slot

  // Sizing
  minWidth?: number;             // px, default 120
  minHeight?: number;            // px, default 80
  defaultWidth?: number | string; // px or '%', default 'auto'
  defaultHeight?: number | string;

  // Resize behavior
  resizable?: {
    top?: boolean;
    bottom?: boolean;
    left?: boolean;
    right?: boolean;
  };                             // Which edges have drag handles. Default: all false (parent controls)

  // Header
  showHeader?: boolean;          // default true
  headerActions?: ReactNode;     // Right side of header (collapse, menu, etc.)
  collapsible?: boolean;         // Show collapse toggle in header
  collapsed?: boolean;           // Controlled collapse state
  onCollapse?: (collapsed: boolean) => void;

  // Status
  status?: 'idle' | 'active' | 'loading' | 'error';  // Header status LED

  // Content
  children: ReactNode;
  className?: string;
}
```

**Visual spec:**

```
┌──────────────────────────────────────────┐
│ ● TERMINAL          [_] [x]             │  ← header: label + status LED + collapse + close
├──────────────────────────────────────────┤
│                                          │
│  (children)                              │
│                                          │
│                                          │
└──────────────────────────────────────────┘
  ↑ resize handle (when resizable.bottom = true)
```

**Resize handle behavior:**
- Default: 1px border, 8px invisible hit area
- Hover: border brightens to `--border-strong`, cursor changes (`col-resize` / `row-resize`)
- Dragging: border becomes `--rose-dim` (2px), guide line appears at cursor position
- Release: animated snap to final position (100ms `--ease-snappy`)

**Snap-to-grid:** Optional `snapGrid` prop (default: off). When set (e.g., `snapGrid={8}`), drag positions snap to nearest multiple. Visual indicator: faint gridlines appear during drag.

**Collapse animation:** Height/width transitions to 0 over 200ms (`--ease-expo`). Header remains visible. Collapsed state shows only header bar (32px tall).

### 2.2 `<PaneGrid>` — Grid Layout Manager

Manages a 2D grid of `<ResizablePane>` children that fill available space.

```tsx
interface PaneGridProps {
  // Layout
  layout: PaneGridLayout;
  minPaneWidth?: number;          // px, default 120
  minPaneHeight?: number;         // px, default 80

  // Persistence
  persistKey?: string;            // localStorage key for saving layout
  onLayoutChange?: (layout: PaneGridLayout) => void;

  children: ReactNode;            // ResizablePane children
  className?: string;
}

interface PaneGridLayout {
  // Row-based layout definition
  rows: PaneGridRow[];
}

interface PaneGridRow {
  height?: number | string;       // px, '%', or 'auto'
  panes: {
    id: string;                   // Matches ResizablePane id
    width?: number | string;      // px, '%', or 'auto'
  }[];
}
```

**Layout example:**
```tsx
<PaneGrid
  layout={{
    rows: [
      { height: '60%', panes: [
        { id: 'editor', width: '50%' },
        { id: 'preview', width: '50%' },
      ]},
      { height: '40%', panes: [
        { id: 'terminal', width: '70%' },
        { id: 'logs', width: '30%' },
      ]},
    ],
  }}
  persistKey="demo-layout"
>
  <ResizablePane id="editor" label="EDITOR"> ... </ResizablePane>
  <ResizablePane id="preview" label="PREVIEW"> ... </ResizablePane>
  <ResizablePane id="terminal" label="TERMINAL"> ... </ResizablePane>
  <ResizablePane id="logs" label="LOGS"> ... </ResizablePane>
</PaneGrid>
```

**Proportional resize:** When a shared border is dragged, both neighbors resize. Other panes in the same row/column adjust proportionally to fill remaining space.

**Collapse behavior:** When a pane collapses, its neighbors expand to fill the freed space (animated, 200ms).

**Persistence:** If `persistKey` is set, layout serializes to `localStorage` on every resize/collapse. On mount, restores from storage. Uses debounced writes (250ms).

### 2.3 `<PaneGroup>` — Linear Pane Stack

A simpler alternative to PaneGrid: a single row or column of panes with shared resize handles between them.

```tsx
interface PaneGroupProps {
  direction: 'horizontal' | 'vertical';
  children: ReactNode;            // ResizablePane children
  className?: string;

  // Sizing
  minPaneSize?: number;           // px, default 80

  // Persistence
  persistKey?: string;

  // Accessibility
  ariaLabel?: string;
}
```

**Handle behavior:**
- Single-click + drag: resize adjacent panes
- Double-click handle: equalize all panes in the group (animated, 300ms `--ease-expo`)
- Keyboard: when handle is focused, Arrow keys resize in 10px increments, Shift+Arrow in 50px

**Visual spec for handle:**
```
Idle:       ··· (3 dot grabber icon, --text-ghost)
Hover:      ··· (--text-dim), cursor: col-resize / row-resize
Focus:      ··· (--rose-dim), focus ring visible
Dragging:   ··· (--rose-bright), 2px guide line
```

**ARIA:** Handles are `role="separator"` with `aria-orientation`, `aria-valuenow` (current size), `aria-valuemin`, `aria-valuemax`.

---

## 3. Loading & State Management Primitives

### 3.1 `<LoadingTransition>` — Pixelated Dither Transition

Wraps content with a CRT-inspired loading transition. When `loading` transitions from true to false, content is progressively revealed through a pixel dither pattern.

```tsx
interface LoadingTransitionProps {
  loading: boolean;
  children: ReactNode;

  // Dither config
  pattern?: 'random' | 'scanline' | 'radial';  // default: 'random'
  duration?: number;             // ms, default 600
  grainSize?: number;            // px per dither cell, default 4
  color?: string;                // Dither overlay color, default '--bg-void'

  // Fallback
  skeleton?: ReactNode;          // What to show while loading (before transition starts)
  className?: string;
}
```

**How it works:**
1. While `loading=true`: shows `skeleton` prop (or `<Skeleton variant="lines" />` by default)
2. When `loading` transitions to `false`:
   - A `<canvas>` overlay covers the content area
   - Canvas is filled with `color` (opaque)
   - Over `duration` ms, pixels are cleared in the chosen pattern, revealing content beneath
   - `random`: pixels clear in random order (Bayer matrix dither)
   - `scanline`: horizontal lines sweep top-to-bottom, each line's pixels clear left-to-right with slight randomization
   - `radial`: pixels clear from center outward in expanding rings
3. Canvas is removed from DOM after transition completes

**Performance:** Canvas is `willReadFrequently: false`. Dither is computed as a pre-shuffled index array on mount (not per-frame). Each animation frame clears a batch of pixels. RequestAnimationFrame, not setInterval.

**Reduced motion:** Instant cut (no dither), content appears immediately.

### 3.2 `<ContentSwitch>` — Animated Content Replacement

Prevents the "blank then content" flash when children change. Crossfades through a skeleton intermediate.

```tsx
interface ContentSwitchProps {
  /** Unique key for current content. When key changes, transition triggers. */
  contentKey: string;
  children: ReactNode;

  // Transition config
  duration?: number;              // ms per phase, default 200 (total = 3x)
  skeleton?: ReactNode;           // Intermediate skeleton. Default: Skeleton variant="lines"
  mode?: 'crossfade' | 'fade-through';  // default: 'fade-through'

  className?: string;
}
```

**Transition sequence (fade-through mode):**
1. Old content fades out (opacity 1 -> 0, `duration` ms)
2. Skeleton appears, shimmers briefly (`duration` ms)
3. New content fades in (opacity 0 -> 1, `duration` ms)

**Crossfade mode:** Old and new overlap, old fades out while new fades in simultaneously. No skeleton intermediate.

**Height transition:** Container smoothly animates height between old and new content sizes (200ms `--ease-expo`). No layout jump.

### 3.3 `<LazyPane>` — Connection-Aware Pane

A pane that shows progressive loading states based on WebSocket/SSE connection lifecycle.

```tsx
interface LazyPaneProps {
  // Connection state
  connectionState: 'connecting' | 'connected' | 'error' | 'disconnected';
  hasData: boolean;               // True once first meaningful data arrives

  // Display
  label?: string;
  children: ReactNode;            // Rendered only when connected + hasData

  // Error recovery
  onReconnect?: () => void;
  retryCountdown?: number;        // Seconds until next auto-retry (displayed to user)

  // Loading config
  skeleton?: ReactNode;
  className?: string;
}
```

**State machine:**
```
connecting  →  Skeleton shimmer + "Connecting..." status text
connected   →  If !hasData: Skeleton shimmer + "Waiting for data..."
              If hasData: children (faded in via LoadingTransition)
error       →  ErrorState panel with message + reconnect button + countdown
disconnected → "Connection lost" banner at top, last content still visible (dimmed)
```

**Visual transitions:**
- `connecting -> connected`: Skeleton continues until `hasData` flips
- `connected (no data) -> connected (data)`: LoadingTransition dither reveal
- `connected -> error`: Red flash on border (400ms), error state fades in
- `error -> connecting`: Error state fades out, skeleton resumes
- `connected -> disconnected`: Top banner slides down (200ms), content dims to 50% opacity

### 3.4 `<ConnectionGuard>` — Connection Gate Wrapper

Higher-order wrapper that blocks rendering until a connection is established.

```tsx
interface ConnectionGuardProps {
  // Connection
  connected: boolean;
  connecting: boolean;
  error?: string | null;

  // Retry
  onRetry?: () => void;
  retryInterval?: number;         // ms between auto-retries, default 5000
  maxRetries?: number;            // default 10

  // Display
  message?: string;               // "Connecting to Roko server..."
  children: ReactNode;
  className?: string;
}
```

**Connecting animation:**
```
┌─────────────────────────────────────────┐
│                                         │
│         ●  ●  ●                         │  ← 3 dots, sequential pulse
│                                         │
│    Connecting to Roko server...         │
│                                         │
│    ███████████░░░░░░░  retry in 3s      │  ← progress bar counting down
│                                         │
│    Attempt 2 of 10                      │
│                                         │
└─────────────────────────────────────────┘
```

**Retry visualization:** Linear progress bar fills from left to right over `retryInterval` ms. When full, retry fires and bar resets. Bar color: `--text-ghost` track, `--rose-dim` fill. Attempt count displayed below.

**Success transition:** Guard fades out (150ms), children fade in via `<Transition enter="fadeUp">`.

---

## 4. Stepped Progress Components

Rich progress indicators for the multi-step orchestration pipeline. All extend or compose with the base `<StepProgress>` from 09.

### 4.1 `<StepProgress>` (Enhanced) — Rich Step Indicator

Extends the 09 base with icon support, descriptions, and richer animations.

```tsx
interface StepProgressProps {
  steps: StepDef[];
  orientation?: 'horizontal' | 'vertical' | 'circular';
  size?: 'sm' | 'md' | 'lg';     // sm=compact, md=default, lg=detailed

  // Animation
  animated?: boolean;             // default true
  showParticles?: boolean;        // Particle trail on active connecting line

  className?: string;
}

interface StepDef {
  id: string;
  label: string;
  description?: string;           // Shown below label (md/lg only)
  icon?: ReactNode;               // Custom icon. Default: number (pending), checkmark (done), X (error)
  status: 'done' | 'active' | 'pending' | 'error' | 'skipped';
  duration?: string;              // "2.3s" — shown as metadata on done steps
}
```

**Visual spec (horizontal, md):**
```
  ✓ Parse       ◉ Compose      ○ Dispatch     ○ Gate        ○ Persist
  ────────────  ═══════════    ···········    ···········   ···········
  0.4s          building...
```
- Done steps: `--status-success` fill, checkmark icon, solid connecting line
- Active step: `--rose-bright` fill, pulsing glow (2.4s cycle), gradient connecting line (green -> rose)
- Pending steps: `--text-ghost` outline, dotted connecting line
- Error steps: `--status-error` fill, X icon, red flash on transition
- Skipped steps: `--text-ghost` fill, dash icon, dashed connecting line

**Connecting line animations:**
- Done: solid line, `--status-success` color
- Active: gradient line from previous step color to `--rose-glow`, with a bright "comet" spot (6px glow) that travels along the line at 2s intervals
- Pending: dotted line, `--text-ghost`

**Step dot sizes:** sm=8px, md=12px, lg=16px. Active dot has 3px glow ring.

**Particle trail (optional):** When `showParticles=true`, the active connecting line has 3-5 small particles (2px dots) drifting along it, color `--rose-glow` at varying opacities. CSS animation, not canvas.

### 4.2 `<GradientStepRail>` — Gradient-Filled Horizontal Rail

A visually rich horizontal progress rail where the connecting line fills with a continuous gradient.

```tsx
interface GradientStepRailProps {
  steps: { id: string; label: string; status: 'done' | 'active' | 'pending' }[];
  progress?: number;              // 0-1, fine-grained progress within active step

  // Visual
  showComet?: boolean;            // Bright spot traveling along gradient. Default true
  cometSpeed?: number;            // seconds per traversal, default 2
  dotSize?: number;               // px, default 16

  className?: string;
}
```

**Visual spec:**
```
  (✓)═══════════(✓)═══════════(◉)───────────(○)───────────(○)
  Parse         Compose       Dispatch      Gate          Persist
```

**Gradient:** Single continuous gradient across all connecting lines:
- Done section: `--status-success` to `--bone`
- Active section: `--bone` to `--rose-glow` (partial fill based on `progress`)
- Pending section: `--text-ghost` (no gradient, flat)

**Comet effect:** A bright spot (8px wide, `--rose-glow` with 20px blur glow) travels along the filled portion of the gradient. CSS animation: `translateX(0%) -> translateX(100%)` over `cometSpeed` seconds, infinite, ease-in-out.

**Step dots:** 16px circles with animated SVG icons inside:
- Done: checkmark draws in (stroke-dashoffset animation, 300ms)
- Active: rotating arc (like Spinner) inside the dot
- Pending: empty circle, `--text-ghost` stroke

### 4.3 `<VerticalTimeline>` — Detailed Vertical Step Timeline

For displaying execution history, plan progress, or event sequences with rich detail per step.

```tsx
interface VerticalTimelineProps {
  steps: TimelineStep[];

  // Scroll
  maxHeight?: string;             // Scrollable if content exceeds. Default: 'none'
  fadeEdges?: boolean;            // Gradient fade top/bottom when scrollable. Default true

  // Detail
  expandable?: boolean;           // Steps can expand to show detail. Default true
  defaultExpanded?: string[];     // IDs of initially expanded steps

  className?: string;
}

interface TimelineStep {
  id: string;
  timestamp?: string;             // ISO string or formatted time
  title: string;
  description?: string;
  detail?: ReactNode;             // Expandable detail content
  status: 'done' | 'active' | 'pending' | 'error' | 'skipped';
  icon?: ReactNode;
  duration?: string;              // "2.3s"
  metadata?: Record<string, string>;  // Key-value pairs shown as chips
}
```

**Visual spec:**
```
  ● 14:32:01  Parse PRD                          2.3s
  │           Extracted 5 tasks from system-prompt-wiring.md
  │           ┌─────────────────────────────────────────┐
  │           │ Task 1: Wire builder into orchestrate   │  ← expandable detail
  │           │ Task 2: Add template selection logic    │
  │           └─────────────────────────────────────────┘
  │
  ◉ 14:32:04  Compose System Prompt                     ← active, pulsing
  │           Building 9-layer prompt for planner role
  │
  ○ --------  Dispatch Agent
  │           Waiting for compose to complete
  │
  ○ --------  Run Gates
  ○ --------  Persist Results
```

**Connecting line:** 2px wide, runs down the left side.
- Done segment: `--status-success`
- Active -> pending boundary: gradient from `--rose-bright` to `--text-ghost`
- Pending segment: dotted, `--text-ghost`

**Active step:** Dot pulses (2.4s cycle, scale 1.0 -> 1.3 -> 1.0). Border: animated `--rose-dim` left border on the step card.

**Expand/collapse:** Detail section slides down (200ms `--ease-expo`), with fade. Chevron icon rotates 90 degrees.

**Scroll fade:** When `fadeEdges=true` and content is scrollable, 40px gradient masks at top and bottom edges (transparent -> `--bg-void`).

### 4.4 `<CircularProgress>` — Radial Progress Ring

SVG-based circular progress for dashboard metrics and phase-level progress.

```tsx
interface CircularProgressProps {
  // Progress
  value: number;                  // 0-100
  segments?: { value: number; color: string; label?: string }[];  // Multi-segment mode

  // Visual
  size?: number;                  // px diameter, default 120
  thickness?: number;             // Stroke width, default 8
  color?: string;                 // Default '--rose-bright'
  trackColor?: string;            // Default '--border-soft'
  gradient?: boolean;             // Gradient stroke. Default false

  // Center content
  centerValue?: ReactNode;        // e.g., <AnimatedNumber value={73} />
  centerLabel?: string;           // e.g., "C-FACTOR"
  centerIcon?: ReactNode;         // Alternative to value (icon or mini chart)

  // Animation
  animated?: boolean;             // Animate sweep on mount/change. Default true
  animationDuration?: number;     // ms, default 600

  className?: string;
}
```

**Visual spec:**
```
       ╭─────────╮
      ╱  ████████ ╲
     │ ██        ██│
     │█    73%    █│     ← center: AnimatedNumber + label
     │ ██        ██│
      ╲  ████████ ╱
       ╰─────────╯
        C-FACTOR
```

**Sweep animation:** `stroke-dashoffset` transitions from full circumference to target value. Easing: `--ease-expo`. Duration: `animationDuration`.

**Gradient stroke:** When `gradient=true`, stroke uses SVG `<linearGradient>` from `--rose-dim` to `--rose-bright` (or segment colors in multi-segment mode).

**Multi-segment mode:** When `segments` is provided, the ring is divided into colored arcs. Each segment fills proportionally. Gaps (2px) between segments. Legend below the ring.

### 4.5 `<MilestoneProgress>` — Linear Progress with Milestone Markers

A progress bar with labeled milestone points that celebrate completion.

```tsx
interface MilestoneProgressProps {
  // Progress
  value: number;                  // 0-100
  milestones: Milestone[];

  // Visual
  height?: number;                // Bar height px, default 8
  color?: string;                 // Fill color, default '--rose-bright'
  showParticles?: boolean;        // Particle effect at leading edge. Default true

  // Animation
  animated?: boolean;             // default true
  onMilestoneReached?: (id: string) => void;

  className?: string;
}

interface Milestone {
  id: string;
  position: number;               // 0-100, where on the bar
  label: string;
  icon?: ReactNode;
  reached?: boolean;              // Override: manually mark as reached
}
```

**Visual spec:**
```
  Parse    Compose    Dispatch       Gate         Persist
    ↓         ↓          ↓            ↓             ↓
  ──●════════●══════════◉────────────○─────────────○──
  0%        20%        45%          70%           100%
                        ↑
                   leading edge (particle glow)
```

**Milestone markers:**
- Unreached: 8px circle, `--text-ghost` outline
- Reached: 12px circle (scale-up 8->12 over 200ms `--ease-bounce`), filled with segment color, glow ring (`box-shadow: 0 0 12px` matching color), label brightens from `--text-ghost` to `--text-primary`

**Leading edge:** The fill bar's right edge has a 4px bright spot (`--rose-glow`) with a soft glow (`0 0 16px`). When `showParticles=true`, 2-3 tiny dots (1-2px) drift upward from the edge (CSS `@keyframes particle-float`).

**Celebration animation:** When progress crosses a milestone:
1. Milestone dot scales 1.0 -> 1.5 -> 1.0 (300ms `--ease-bounce`)
2. Brief glow pulse on the dot (400ms)
3. `onMilestoneReached` callback fires

---

## 5. Three.js / WebGL Background Primitives

Ambient expression layers rendered behind content. All share a common performance contract: 30fps cap, GPU-only rendering, `<canvas>` with `pointer-events: none`, and `prefers-reduced-motion` degradation to static CSS.

### 5.1 `<ParticleField>` — Generalized Particle Background

Generalizes the existing `HeroParticleField` into a reusable background primitive.

```tsx
interface ParticleFieldProps {
  // Particles
  particleCount?: number;         // default 80
  colors?: string[];              // CSS color strings, default: ['--rose', '--bone', '--dream']
  particleSize?: [number, number]; // [min, max] px, default [1, 3]

  // Motion
  speed?: number;                 // Drift speed multiplier, default 1.0
  connectionDistance?: number;    // px, draw line between particles closer than this. Default 120
  connectionOpacity?: number;     // 0-1, default 0.08

  // Interaction
  reactToMouse?: boolean;         // Particles avoid/attract to cursor. Default false
  mouseRadius?: number;           // px influence radius, default 100
  mouseForce?: 'attract' | 'repel'; // default 'repel'
  reactToScroll?: boolean;        // Speed up on scroll. Default false

  // State reactivity
  density?: number;               // 0-2, multiplier on particleCount based on activity. Default 1
  colorShift?: string;            // CSS color to shift palette toward (e.g., '--status-error' on failures)

  // Container
  className?: string;
  style?: CSSProperties;
}
```

**Rendering:** WebGL via raw `<canvas>` (no Three.js dependency for particles). Draws points + lines. Uses `Float32Array` for positions, velocities. Single draw call per frame via `gl.drawArrays`.

**State reactivity:** When `density` changes, particles smoothly spawn/despawn (fade in/out over 500ms). When `colorShift` changes, all particle colors lerp toward the new color over 1s.

**Reduced motion:** Particles rendered as static dots (no animation). Connections drawn once.

### 5.2 `<NoiseBackground>` — Shader Noise Background

WebGL fragment shader that renders evolving simplex noise.

```tsx
interface NoiseBackgroundProps {
  // Noise
  type?: 'simplex' | 'perlin' | 'worley';    // default 'simplex'
  scale?: number;                // Noise frequency, default 3.0
  speed?: number;                // Evolution speed, default 0.3
  octaves?: number;              // Fractal octaves, default 3

  // Color
  colorA?: string;               // Low value color, default '--bg-void'
  colorB?: string;               // High value color, default '--rose-deep'
  colorC?: string;               // Optional mid-blend color, default '--bg-raised'

  // Reactivity
  frequencyMultiplier?: number;  // 1.0 = normal, >1 = more active. Drives noise scale dynamically
  colorShift?: string;           // Override colorB temporarily (e.g., toward error)

  className?: string;
  style?: CSSProperties;
}
```

**Shader uniforms:**
```glsl
uniform float u_time;
uniform float u_scale;
uniform float u_speed;
uniform vec3 u_colorA;            // Parsed from CSS var at init
uniform vec3 u_colorB;
uniform vec3 u_colorC;
uniform float u_frequencyMult;    // Animated when prop changes
```

**Performance:** Single fullscreen quad, single fragment shader. No vertex transforms. Renders at 30fps (or less if tab is background). Total GPU cost: negligible.

**Reduced motion:** Static noise texture, no animation. Rendered once on mount.

### 5.3 `<FluidGradient>` — Animated Gradient Mesh

A mesh of control points with interpolated gradients that drift organically.

```tsx
interface FluidGradientProps {
  // Mesh
  points?: number;                // Control points, default 5
  colors?: string[];              // Colors at control points, default ROSEDUST palette subset

  // Motion
  speed?: number;                 // Drift speed, default 0.5
  turbulence?: number;            // How erratic the motion is, default 0.3

  // Reactivity
  pulse?: boolean;                // When true, gradient surges (speed * 3 briefly). Default false
  colorOverride?: string[];       // Temporarily replace palette (lerps over 1s)

  className?: string;
  style?: CSSProperties;
}
```

**Rendering:** WebGL fragment shader with N uniform vec2 control points + vec3 colors. Shader computes weighted blend based on distance to each point. Points drift via simplex noise in JS, positions uploaded per frame.

**Pulse behavior:** When `pulse` transitions to true, all points accelerate (speed * 3) for 400ms, then decelerate back. Creates a visible surge effect.

### 5.4 `<HeartbeatLine>` — EKG Waveform Decoration

Canvas-based scrolling waveform that pulses like a heartbeat.

```tsx
interface HeartbeatLineProps {
  // Timing
  speed?: 'fast' | 'medium' | 'glacial';  // 0.7s, 3s, 300s cycle
  bpm?: number;                    // Override: beats per minute (overrides speed)

  // Visual
  color?: string;                  // Stroke color, default '--rose'
  thickness?: number;              // px, default 2
  height?: number;                 // Canvas height px, default 40
  glowIntensity?: number;         // 0-1, glow behind peaks. Default 0.3

  // Reactivity
  amplitude?: number;              // 0-1, pulse height. Maps to agent affect. Default 0.5
  colorOverride?: string;          // Temporary color shift (e.g., error -> red)

  // Layout
  orientation?: 'horizontal' | 'vertical';
  className?: string;
  style?: CSSProperties;
}
```

**Waveform shape:** Classic EKG pattern: flat baseline -> small P wave -> sharp QRS spike -> small T wave -> flat. Repeats at `bpm` rate. Between beats, line is flat with very slight noise.

**Scrolling:** Canvas content scrolls right-to-left. New waveform data appended to right edge. Old data scrolls off left edge. Scroll speed matches pulse rate.

**Three timescales (when used for agent heartbeat):**
- `fast` (0.7s): Color saturation pulses. Maps to emotion/PAD pleasure axis
- `medium` (3s): Amplitude/brightness pulses. Maps to agent health
- `glacial` (300s): Very slow hue drift across the stroke color. Maps to lifecycle phase

**Reduced motion:** Static flat line with periodic dots at beat positions. No scrolling.

### 5.5 `<GlitchOverlay>` — Visual Corruption Effect

Canvas overlay that adds scanline glitch effects, triggered by events or continuous at low intensity.

```tsx
interface GlitchOverlayProps {
  // Intensity
  intensity?: number;              // 0-1, 0=off, 1=maximum corruption. Default 0
  mode?: 'continuous' | 'burst';   // continuous = always on at intensity; burst = triggers once then fades

  // Burst config (mode='burst')
  burstDuration?: number;          // ms, default 400
  burstDecay?: number;             // ms to fade after burst, default 600

  // Visual
  scanlineHeight?: number;         // px, default 2
  rgbSplitMax?: number;           // Max px of RGB channel separation, default 4
  chromaticAberration?: boolean;   // Enable chromatic aberration on intense glitches. Default true

  className?: string;
  style?: CSSProperties;
}
```

**Effects (applied in layers based on intensity):**

| Intensity | Effects |
|-----------|---------|
| 0.0-0.2   | Occasional single scanline shift (1-2px horizontal displacement), every 3-5s |
| 0.2-0.5   | Multiple scanlines shift, RGB split (1-2px), every 1-2s |
| 0.5-0.8   | Frequent scanline blocks (8-16px tall), RGB split (2-4px), flickering |
| 0.8-1.0   | Heavy corruption: large block shifts, strong RGB split, chromatic aberration, opacity flicker |

**Burst mode:** On trigger, intensity spikes to prop value, then decays exponentially over `burstDecay` ms back to 0. Used for: gate failure events, connection drops, error states.

**Rendering:** 2D canvas overlay. Each frame: copy underlying content pixels (via `getImageData` or `drawImage` from a snapshot), apply displacement/color transforms, draw to overlay canvas. At low intensity, only runs every ~3s (not every frame).

**Reduced motion:** Disabled entirely. No visual corruption.

### 5.6 `<AmbientContainer>` — Container with Built-In Background

Convenience wrapper that combines a content container with a WebGL background layer.

```tsx
interface AmbientContainerProps {
  // Background
  background: 'noise' | 'particles' | 'fluid' | 'heartbeat';
  backgroundProps?: Partial<NoiseBackgroundProps | ParticleFieldProps | FluidGradientProps | HeartbeatLineProps>;

  // Reactivity
  activity?: number;              // 0-1, drives background intensity
  status?: 'idle' | 'active' | 'error';  // Drives background color shift

  // Container (delegates to Panel)
  label?: string;
  variant?: 'glass' | 'solid' | 'outline' | 'ghost';
  children: ReactNode;
  className?: string;
}
```

**Status -> background mapping:**
- `idle`: Low intensity, muted colors, slow animation
- `active`: Higher intensity, brighter colors, faster animation. Background `activity` prop = 0.6
- `error`: Color shifts toward `--status-error`, intensity pulses

**Layering:** WebGL canvas renders behind children at `z-index: 0`. Content at `z-index: 1` with `position: relative`. Glass variant Panel on top provides the frosted overlay.

---

## 6. Agent-Namespaced Components

Components that subscribe to per-agent events and express agent-specific state. All require an `agentId` prop and consume data from DataHub's agent state slice.

**Layout context:** Agent containers are dense cards within the normal page flow -- NOT full-screen panels. An `<AgentContainer>` participates in the page scroll like any other card component. Its internal feeds and logs use `max-height` with internal scroll and fade gradients, but the container itself has content-determined height. Multiple agent containers can stack vertically or tile in a grid, all within the scrollable page.

### 6.1 `<AgentContainer>` — Agent-Scoped Wrapper

The primary container for displaying agent-specific content. Combines Panel, heartbeat decoration, and state-reactive background.

```tsx
interface AgentContainerProps {
  agentId: string;

  // Display options
  showHeader?: boolean;           // Agent name, role, status LED. Default true
  showHeartbeat?: boolean;        // HeartbeatLine decoration. Default false
  heartbeatPosition?: 'top' | 'bottom'; // Default 'bottom'
  showMetrics?: boolean;          // AgentMetricBar below header. Default false
  showFeed?: boolean;             // AgentFeed at bottom. Default false

  // Background
  ambientBackground?: boolean;    // State-reactive background. Default false
  backgroundType?: 'noise' | 'particles' | 'fluid';

  // Container
  variant?: 'glass' | 'solid' | 'outline';
  children: ReactNode;
  className?: string;
}
```

**Header spec:**
```
┌─────────────────────────────────────────────────┐
│ ● planner  [ROLE: architect]  ◉ active    3.2s  │
├─────────────────────────────────────────────────┤
│  C: 0.73  │  N: 142  │  PAD: +0.3  │  $0.042   │  ← AgentMetricBar (optional)
├─────────────────────────────────────────────────┤
│                                                  │
│  (children)                                      │
│                                                  │
├─────────────────────────────────────────────────┤
│  ▁▁▁█▁▁▁▁▁█▁▁▁▁▁█▁▁▁▁▁█▁▁▁                    │  ← HeartbeatLine (optional)
└─────────────────────────────────────────────────┘
```

**State-reactive styling:**
- Border color: maps to lifecycle phase. Thriving = `--rose-bright`, declining = `--rose-dim`, dormant = `--text-ghost`
- Background warmth: maps to PAD pleasure. High pleasure = warmer (rose tint), low = cooler (blue-grey tint). Implemented via background color opacity shifts
- Header status LED: maps to agent status (`active` = pulsing green, `idle` = dim, `error` = red pulse)

### 6.2 `<AgentMetricBar>` — Per-Agent Metric Strip

Compact metric display for a single agent.

```tsx
interface AgentMetricBarProps {
  agentId: string;

  // Metric selection
  metrics?: ('c-factor' | 'neuro-density' | 'pad' | 'tokens' | 'cost' | 'task')[];
  // Default: all

  // Display
  compact?: boolean;              // Single row with dividers vs wrapped grid. Default true
  flashOnChange?: boolean;        // AnimatedNumber flash. Default true

  className?: string;
}
```

**Metrics displayed:**

| Metric | Label | Format | Color |
|--------|-------|--------|-------|
| C-factor | `C:` | `0.73` | `--bone` (normal), `--status-error` (<0.3) |
| Neuro density | `N:` | `142` | `--dream` |
| PAD | `PAD:` | `+0.3/-0.1/+0.5` | Gradient based on pleasure value |
| Tokens | `Tok:` | `12.4k` | `--text-soft` |
| Cost | `$` | `$0.042` | `--bone` (normal), `--warning` (>$0.10) |
| Active task | `Task:` | Truncated task name | `--text-primary` |

**Update behavior:** All values use `<AnimatedNumber>`. On change: spring animation (300ms) + flash highlight (`--bone-bright` text glow, 300ms fade).

**Data source:** Subscribes to `DataHub.agents[agentId].metrics` (or equivalent SSE per-agent event channel).

### 6.3 `<AgentFeed>` — Live Agent Event Feed

Scrolling event feed for a specific agent's activity.

```tsx
interface AgentFeedProps {
  agentId: string;

  // Display
  maxEvents?: number;             // Keep last N events in DOM. Default 100
  maxHeight?: string;             // Scrollable height. Default '300px'
  showTimestamps?: boolean;       // Default true

  // Filtering
  eventTypes?: ('inference' | 'gate' | 'tool' | 'somatic' | 'error' | 'lifecycle')[];
  // Default: all

  className?: string;
}
```

**Event rendering by type:**

| Type | Icon | Color | Content |
|------|------|-------|---------|
| inference | `>_` | `--bone` | "claude-opus-4 | 1.2k tok | 2.3s | $0.04" |
| gate | `◉` / `✗` | `--status-success` / `--status-error` | "compile: PASS (0.4s)" |
| tool | `⚙` | `--dream` | "read_file: src/main.rs (142 lines)" |
| somatic | `♦` | `--rose` | "curiosity +0.2, confidence -0.1" |
| error | `!` | `--status-error` | Error message, mono font |
| lifecycle | `↻` | `--text-dim` | "agent spawned", "task assigned: T3" |

**Scroll behavior:**
- Auto-scrolls to bottom when new events arrive (if user is at or near bottom)
- When user scrolls up: auto-scroll pauses, "N new events" badge appears at bottom
- Click badge: smooth scroll to bottom, resume auto-scroll
- Fade edges: 24px gradient masks at top/bottom when scrollable

**Event entrance:** New events fade in from bottom (opacity 0 -> 1, 150ms). Events are `<Transition enter="fadeIn">`.

### 6.4 `<AgentHeartbeat>` — Three-Timescale Heartbeat

A visual decoration that expresses agent state through three concurrent animation timescales.

```tsx
interface AgentHeartbeatProps {
  agentId: string;

  // Visual mode
  mode?: 'glow' | 'border' | 'line';
  // glow: radial glow behind container
  // border: animated border color/width
  // line: HeartbeatLine waveform

  // Size
  size?: number;                   // px, only for 'glow' mode. Default: 60

  // Manual override (instead of auto-subscribing to agent state)
  emotion?: number;                // -1 to 1, maps to fast pulse saturation
  health?: number;                 // 0 to 1, maps to medium pulse brightness
  lifecycle?: number;              // 0 to 1, maps to glacial hue drift

  className?: string;
}
```

**Three timescales:**

| Timescale | Period | Visual | Maps to |
|-----------|--------|--------|---------|
| Fast | 0.7s | Color saturation pulse | PAD pleasure axis (-1..+1). High pleasure = saturated rose. Low = desaturated grey |
| Medium | 3-5s | Opacity/brightness pulse | Agent health (0..1). High = bright, crisp pulse. Low = dim, irregular pulse |
| Glacial | 5+ min | Background hue drift | Lifecycle phase (0..1). 0 = cold blue-grey hue, 1 = warm rose-gold hue |

**Glow mode:** Renders as a radial gradient `<div>` behind the target container. Size = `size` px. Center opacity pulses at fast rate. Spread radius pulses at medium rate. Hue drifts at glacial rate. CSS `filter: blur()` for soft edge.

**Border mode:** Renders as an animated `box-shadow` on the parent container. Shadow color saturation = fast timescale. Shadow spread = medium timescale. Shadow hue = glacial timescale.

**Line mode:** Delegates to `<HeartbeatLine>`. Amplitude = medium timescale. Color saturation = fast timescale. Base color hue = glacial timescale.

### 6.5 `<AgentAvatar>` — Procedural Agent Identity

Simplified web version of the bardo Spectre creature system. A procedural visual identity for each agent.

```tsx
interface AgentAvatarProps {
  agentId: string;

  // Appearance
  size?: number;                   // px, default 48
  variant?: 'dot-cloud' | 'glyph' | 'ring';
  // dot-cloud: particle cluster with density/color from state
  // glyph: single SVG icon from emotion archetype
  // ring: concentric rings with state-reactive properties

  // State (auto-subscribed from DataHub if not provided)
  emotion?: number;                // -1..+1 -> eye glyph + color
  health?: number;                 // 0..1 -> density/opacity
  lifecycle?: number;              // 0..1 -> complexity

  // Interaction
  onClick?: () => void;
  tooltip?: boolean;               // Show agent name + status on hover. Default true

  className?: string;
}
```

**Dot-cloud variant (default):**
- N particles (20-50) arranged in a rough circular cluster
- Particle count = `lifecycle * 50` (more particles = more alive)
- Particle color: hue from `emotion` (-1 = cool blue, 0 = neutral grey, +1 = warm rose)
- Particle opacity: `health * 0.8 + 0.2` (always at least 20% visible)
- Particles drift via simplex noise (always in motion, subtle)
- Rendered as SVG `<circle>` elements with CSS transitions (not canvas, for accessibility)

**Glyph variant:**
- Single SVG icon chosen from emotion archetype:
  - Confident (+pleasure, +dominance): upward chevron
  - Curious (+arousal, neutral): rotating dots
  - Cautious (-pleasure, -dominance): inward-pointing marks
  - Focused (+arousal, +dominance): crosshair
  - Resting (low arousal): horizontal line
- Icon color from PAD modulation
- Subtle pulse animation matching fast heartbeat timescale

**Ring variant:**
- 2-3 concentric rings
- Outer ring: radius based on `lifecycle`, color from `health`
- Middle ring: rotation speed from PAD arousal
- Inner dot: color from PAD pleasure

---

## 7. Event Feed / Activity Stream Primitives

**Layout context:** Event feeds render as compact scrollable sections within the page flow. They are NOT dedicated fixed panels. Every feed component uses `max-height` (default 300-400px) with internal `overflow-y: auto` and gradient fade at top/bottom edges. They sit alongside other content in the scrollable page. Auto-scroll to bottom when the user is at the bottom; pause auto-scroll and show "N new events" badge when the user scrolls up.

### 7.1 `<EventStream>` — Universal Real-Time Event Feed

Subscribes to `ServerEvent` types from the DataHub transport layer and renders them as a scrolling feed.

```tsx
interface EventStreamProps {
  // Data source
  eventTypes?: string[];           // ServerEvent type filter. Default: all
  source?: 'global' | 'agent' | 'bench' | 'pipeline';  // Which event channel
  agentId?: string;                // Required when source='agent'

  // Display
  maxEvents?: number;              // Default 200
  maxHeight?: string;              // Default '400px'
  showTimestamps?: boolean;        // Default true
  groupByType?: boolean;           // Collapse consecutive same-type events. Default false

  // Filtering
  filterBar?: boolean;             // Show filter chips above feed. Default false

  className?: string;
}
```

**Event type renderers (pluggable):**

| Event type | Renderer | Visual |
|------------|----------|--------|
| `inference.start` | Metric row | Model name, token count, streaming indicator |
| `inference.complete` | Metric row | Duration, cost, token velocity |
| `gate.*` | Status badge | Gate name + pass/fail badge, duration |
| `task.*` | Timeline entry | Task name, status change, agent |
| `error` | Alert row | Red left border, error message, stack toggle |
| `lifecycle.*` | System note | Dim text, lifecycle state change |
| `somatic.*` | Emotion chip | PAD delta, somatic marker type |

**Auto-scroll:** Same behavior as AgentFeed (auto-scroll when at bottom, "N new" badge when scrolled up).

**Filter bar:** When enabled, shows `<Chip>` toggles for each event type above the feed. Active filters highlight; inactive dim. Toggling re-renders the feed with filtered events.

### 7.2 `<BlockFeed>` — Chain Block Arrival Visualization

For the chain/blockchain integration (Phase 2+). Renders block arrivals with decorative emphasis.

```tsx
interface BlockFeedProps {
  blocks: Block[];
  maxBlocks?: number;              // Keep last N. Default 20
  maxHeight?: string;              // Default '300px'
  className?: string;
}

interface Block {
  number: number;
  hash: string;
  timestamp: string;
  txCount: number;
  gasUsed?: string;
  witnessCount?: number;           // Roko-specific: number of witnessed events in this block
}
```

**Block arrival animation:**
1. Block number fades in at 48px font size (`--text-3xl`, `--bone-bright`), centered
2. Over 600ms: shrinks to 14px and slides into the list at top position
3. Hash truncated: `0xab12...ef34` with copy button (click -> "Copied!" toast)
4. Pulse on container border (200ms `--bone-dim` flash) on each new block arrival

**List entry:**
```
  #18,234,501  0xab12...ef34  │  142 tx  │  2.1M gas  │  3 witnesses  │  2s ago
```

### 7.3 `<InferenceFeed>` — LLM Inference Event Feed

Specialized feed for LLM inference events with model-specific coloring and inline metrics.

```tsx
interface InferenceFeedProps {
  // Data
  events: InferenceEvent[];
  maxEvents?: number;              // Default 50

  // Display
  showSparkline?: boolean;         // Inline token velocity sparkline. Default true
  showCost?: boolean;              // Default true
  groupByModel?: boolean;          // Group consecutive same-model calls. Default false

  maxHeight?: string;
  className?: string;
}

interface InferenceEvent {
  id: string;
  model: string;                   // e.g., "claude-opus-4"
  provider: string;                // e.g., "anthropic"
  tokensIn: number;
  tokensOut: number;
  duration: number;                // ms
  cost: number;                    // USD
  status: 'streaming' | 'complete' | 'error';
  tokenVelocity?: number[];       // Tokens per 100ms for sparkline
  timestamp: string;
}
```

**Per-event rendering:**
```
  ◉ claude-opus-4    1,234 in / 856 out    2.3s    $0.042    ▃▅▇█▆▃▁
                                                               ↑ sparkline
```

**Model-specific colors:** Each model gets a consistent color from a hash of its name, drawn from the palette in `lib/palette.ts`. The color appears as the left border accent and the status dot.

**Streaming state:** When `status='streaming'`, the event row has a pulsing left border and the token counts increment via `<AnimatedNumber>`.

---

## 8. Floating Chat / Agent Interaction

### 8.1 `<FloatingChat>` — Draggable Agent Chat Widget

An Intercom-style floating chat window for sending messages to agents.

```tsx
interface FloatingChatProps {
  // Agent
  agents: { id: string; name: string; role: string; status: string }[];
  defaultAgent?: string;           // agentId to start with

  // Position
  defaultPosition?: { x: number; y: number };  // Default: bottom-right corner
  persistPosition?: boolean;       // Save to localStorage. Default true

  // State
  open?: boolean;
  onOpenChange?: (open: boolean) => void;

  // Callbacks
  onSendMessage?: (agentId: string, message: string) => void;
  onClose?: () => void;

  className?: string;
}
```

**Visual spec (open):**
```
┌─ CHAT ── planner ▾ ──────────── [_] [x] ─┐
│                                            │
│  ┌─ planner ─────────────────────────────┐ │
│  │ I've parsed the PRD and extracted 5   │ │
│  │ tasks. Starting with the system       │ │
│  │ prompt builder integration.           │ │
│  └───────────────────────────────────────┘ │
│                                            │
│  ┌─ you ─────────────────────────────────┐ │
│  │ Sounds good. Prioritize the gate      │ │
│  │ pipeline wiring.                      │ │
│  └───────────────────────────────────────┘ │
│                                            │
│  ┌─ planner ─────────────────────────────┐ │
│  │ Understood. Adjusting task order.     │ │
│  │                                       │ │
│  │ ▸ Tool: reorder_tasks (click to       │ │  ← expandable tool call
│  │   expand)                             │ │
│  └───────────────────────────────────────┘ │
│                                            │
├────────────────────────────────────────────┤
│  Type a message...                    [↵]  │
└────────────────────────────────────────────┘
```

**Features:**
- **Draggable:** Title bar is drag handle. Position clamped to viewport bounds. Position saved to localStorage on drop.
- **Minimizable:** Collapse to icon in corner (32px circle with agent avatar + unread badge).
- **Agent selector:** Dropdown in title bar lists available agents with status indicators.
- **Message thread:** Scrollable message list. Agent messages left-aligned, user messages right-aligned. Streaming responses show typing indicator (3 pulsing dots).
- **Tool call blocks:** Rendered as collapsible `<Panel variant="outline" size="sm">` blocks within messages. Click to expand tool input/output.
- **Resize:** Bottom-right corner resize handle. Min size: 280x300. Max: 50vw x 70vh.

**Minimized state:**
```
  (●) 2        ← circle with agent glyph + unread count badge
```

**Animations:**
- Open: scale 0.9 -> 1.0 + fade in (200ms `--ease-snappy`)
- Minimize: scale 1.0 -> 0.3 + move to corner (300ms `--ease-expo`)
- New message: message slides up from bottom of thread (150ms)
- Unread badge: scale 0 -> 1.0 (200ms `--ease-bounce`)

### 8.2 `<ChatBubble>` — Inline Chat Trigger

A small trigger icon that opens the FloatingChat, positioned relative to agent containers.

```tsx
interface ChatBubbleProps {
  agentId: string;
  unreadCount?: number;
  onClick?: () => void;
  size?: 'sm' | 'md';             // sm=24px, md=32px
  className?: string;
}
```

**Visual:** Circular button with chat icon. `--glass-bg` background, `--border` ring. Hover: scale 1.05, border brightens. Unread count: small red badge (12px circle) at top-right with count number.

**Placement:** Typically positioned `absolute` within an `<AgentContainer>`, top-right corner.

---

## 9. Modal & Overlay System

Extends the basic Modal/Drawer from 09 with richer variants.

### 9.1 `<Modal>` (Enhanced) — Accessible Modal Dialog

Extends 09's Modal with additional features.

```tsx
interface ModalProps {
  open: boolean;
  onClose: () => void;

  // Size
  size?: 'sm' | 'md' | 'lg' | 'xl' | 'full';
  // sm=400px, md=600px, lg=900px, xl=1100px, full=calc(100vw - 64px)

  // Content
  title?: string;
  subtitle?: string;
  icon?: ReactNode;                // Title icon
  children: ReactNode;
  footer?: ReactNode;              // Fixed footer (for action buttons)

  // Behavior
  closeOnBackdrop?: boolean;       // Default true
  closeOnEscape?: boolean;         // Default true
  preventScroll?: boolean;         // Lock body scroll. Default true

  // Accessibility
  ariaLabel?: string;
  ariaDescribedBy?: string;
  initialFocusRef?: RefObject<HTMLElement>;

  className?: string;
}
```

**Backdrop:** `backdrop-filter: blur(4px); background: rgba(6, 6, 8, 0.72)`. Dims background to ~40% brightness.

**Focus trap:** Tab cycles within modal. Initial focus: first focusable element (or `initialFocusRef`). On close, focus returns to trigger element.

**Animation:**
- Enter: backdrop fades 0->1 (150ms). Modal: opacity 0->1 + scale 0.95->1.0 + translateY(8px)->0 (200ms `--ease-snappy`)
- Exit: modal: opacity 1->0 + scale 1.0->0.95 + translateY(0)->8px (150ms). Backdrop fades 1->0 (100ms)

**Stacking:** Multiple modals stack (z-index increments). Each successive backdrop is lighter. Only top modal receives keyboard events.

### 9.2 `<Drawer>` (Enhanced) — Slide-In Panel

Extends 09's Drawer.

```tsx
interface DrawerProps {
  open: boolean;
  onClose: () => void;

  // Position
  position?: 'left' | 'right' | 'bottom';

  // Size
  width?: string;                  // For left/right. Default '400px'
  height?: string;                 // For bottom. Default '50vh'
  maxWidth?: string;               // Default '90vw'
  resizable?: boolean;             // Draggable edge to resize. Default false

  // Content
  title?: string;
  children: ReactNode;
  footer?: ReactNode;

  // Behavior
  backdrop?: boolean;              // Show backdrop. Default true
  pushContent?: boolean;           // Push page content instead of overlaying. Default false

  className?: string;
}
```

**Push mode:** When `pushContent=true`, the main content area translates by the drawer width (left/right) or height (bottom). No backdrop. Content remains interactive.

**Resize (when enabled):** The edge opposite to `position` (e.g., left edge when `position='right'`) has a drag handle (same visual as PaneGroup handles).

**Animation:** Slide from edge (200ms `--ease-expo`). In push mode, content translates simultaneously.

### 9.3 `<CommandPalette>` — Quick-Find Overlay

Cmd+K (Mac) / Ctrl+K (Win) fuzzy-search overlay.

```tsx
interface CommandPaletteProps {
  open: boolean;
  onClose: () => void;
  onSelect: (item: CommandItem) => void;

  // Items
  items: CommandItem[];
  recentItems?: CommandItem[];     // Shown before search input
  pinnedItems?: CommandItem[];     // Always shown at top

  // Customization
  placeholder?: string;            // Default "Search pages, agents, runs..."
  maxResults?: number;             // Default 10
  groupBy?: (item: CommandItem) => string;  // Group items under headers

  className?: string;
}

interface CommandItem {
  id: string;
  label: string;
  description?: string;
  icon?: ReactNode;
  category?: string;               // "Pages", "Agents", "Runs", "Commands"
  shortcut?: string;               // "Cmd+O" — displayed right-aligned
  action: () => void;
}
```

**Visual spec:**
```
┌─────────────────────────────────────────────────┐
│  🔍  Search pages, agents, runs...              │
├─────────────────────────────────────────────────┤
│  RECENT                                         │
│    ◈  Orchestrate — Demo page                   │
│    ◈  planner — Agent detail                    │
│                                                  │
│  PAGES                                  ⌘1-⌘5   │
│    ▸  Orchestrate                                │
│    ▸  Observe                                    │
│    ▸  Evaluate                                   │
│    ▸  Build                                      │
│                                                  │
│  COMMANDS                                        │
│    ⚡  Start benchmark run              ⌘⇧B      │
│    ⚡  Clear terminal                   ⌘K       │
└─────────────────────────────────────────────────┘
```

**Search:** Fuzzy matching on `label` + `description` + `category`. Results ranked by relevance. As user types, results filter and reorder with layout animation.

**Keyboard:** Arrow keys navigate. Enter selects. Escape closes. Type-ahead focus: input always focused.

**Animation:**
- Open: backdrop blur fades in (100ms), palette drops from top with slight scale (0.98->1.0, 200ms `--ease-snappy`)
- Close: reverse (150ms)
- Result items: stagger fade-in (20ms per item, max 10)

---

## 10. Robust Layout Primitives

### 10.1 `<StickyTopLayout>` — Full-Page Layout Shell

Ensures TopNav stays visible while content scrolls naturally.

```tsx
interface StickyTopLayoutProps {
  nav: ReactNode;                  // TopNav component
  children: ReactNode;             // Page content
  className?: string;
}
```

**CSS architecture:**
```css
.sticky-top-layout {
  display: flex;
  flex-direction: column;
  height: 100vh;                   /* or 100dvh for mobile */
}

.sticky-top-layout__nav {
  position: sticky;
  top: 0;
  z-index: var(--z-nav, 100);
  flex-shrink: 0;
}

.sticky-top-layout__content {
  flex: 1;
  overflow-y: auto;
  -webkit-overflow-scrolling: touch;
}
```

**Key behavior:** No `overflow: hidden` on `<body>`. Content area has its own scroll context. TopNav stays pinned. This is the outermost layout wrapper; all pages render inside `__content`.

### 10.2 `<MasonryGrid>` — Variable-Height Card Grid

Pinterest-style layout for variable-height content (knowledge cards, episode entries, research results).

```tsx
interface MasonryGridProps {
  // Layout
  columns?: number;                // Fixed column count. Default: responsive
  minColumnWidth?: number;         // px, for responsive column count. Default 280
  gap?: number;                    // px, default 16

  // Animation
  animateLayout?: boolean;         // Animate position changes. Default true

  children: ReactNode;
  className?: string;
}
```

**Column calculation:** When `columns` is not set: `Math.floor(containerWidth / minColumnWidth)`, clamped to [1, 6].

**Layout algorithm:** Items placed into shortest column. Position computed via JS (not CSS columns, which reorder items). Each item gets `position: absolute` with computed `top` and `left`. Container height set to tallest column.

**Animation:** When layout recalculates (resize, item add/remove), items transition to new positions (200ms `--ease-expo`). New items fade in (150ms delay after position set).

**Responsive:** Re-layouts on container resize (via `ResizeObserver`, debounced 100ms).

### 10.3 `<TreeView>` — Hierarchical Expandable Tree

For plan task hierarchies, knowledge graph navigation, file trees.

```tsx
interface TreeViewProps<T> {
  data: TreeNode<T>[];
  renderNode: (node: TreeNode<T>, depth: number) => ReactNode;

  // State
  expanded?: Set<string>;          // Controlled expanded state
  onExpand?: (id: string, expanded: boolean) => void;
  defaultExpanded?: string[];      // IDs to expand on mount

  // Visual
  showGuides?: boolean;            // Indent guide lines. Default true
  guideStyle?: 'solid' | 'dotted' | 'dashed';  // Default 'dotted'
  indent?: number;                 // px per depth level. Default 20

  // Interaction
  selectable?: boolean;
  selected?: string;
  onSelect?: (id: string) => void;

  className?: string;
}

interface TreeNode<T> {
  id: string;
  data: T;
  children?: TreeNode<T>[];
  status?: 'done' | 'active' | 'pending' | 'error';  // Optional status indicator
}
```

**Visual spec:**
```
  ▸ Plan: system-prompt-wiring           ● active
  ┊  ▾ Phase 1: Research                 ✓ done
  ┊  ┊  ├─ T1: Analyze existing code     ✓
  ┊  ┊  ├─ T2: Map dependencies          ✓
  ┊  ┊  └─ T3: Draft integration plan    ✓
  ┊  ▸ Phase 2: Implementation           ◉ active
  ┊  ▸ Phase 3: Validation               ○ pending
```

**Indent guides:** 1px lines (`--border-soft`) running vertically at each indent level. Style matches `guideStyle` prop. Guide connects to node via horizontal branch (`├─` or `└─` for last child).

**Expand/collapse animation:** Children container slides down from 0 height (200ms `--ease-expo`). Chevron rotates 0 -> 90 degrees.

**Status indicators:** Small dot/icon at right edge of each node, colored by status. Same colors as StepProgress.

**Keyboard:** Arrow Up/Down navigates. Arrow Right expands / enters children. Arrow Left collapses / goes to parent. Enter selects. Home/End jump to first/last visible node.

**ARIA:** `role="tree"`, `role="treeitem"`, `aria-expanded`, `aria-level`, `aria-selected`.

### 10.4 `<VirtualList>` — Virtualized Scrolling List

Only renders visible items plus a buffer for smooth scrolling. For event feeds, large tables, episode lists.

```tsx
interface VirtualListProps<T> {
  items: T[];
  renderItem: (item: T, index: number) => ReactNode;
  keyExtractor: (item: T) => string;

  // Sizing
  estimatedItemHeight?: number;    // px, default 40. Used for initial scroll calculations
  overscan?: number;               // Extra items to render above/below viewport. Default 5

  // Scroll
  maxHeight: string;               // Container height (required for virtualization)
  onEndReached?: () => void;       // Infinite scroll callback
  endReachedThreshold?: number;    // px from bottom to trigger. Default 200

  // Features
  stickyIndices?: number[];        // Items that stick to top when scrolled past
  fadeEdges?: boolean;             // Gradient masks. Default true

  className?: string;
}
```

**Virtualization approach:** Uses `IntersectionObserver` for visibility tracking (not scroll position math). Each item wrapped in a sentinel `<div>`. Only items intersecting the viewport (plus `overscan`) are rendered. Non-visible items replaced with spacer divs of measured height.

**Dynamic height:** Items can be variable height. After first render, actual heights are measured and cached. Scroll position corrected if measured heights differ from estimates.

**Infinite scroll:** `onEndReached` fires when the last `endReachedThreshold` px of content become visible. Consumer handles loading more items. While loading, a `<Skeleton>` row appears at the bottom.

**Performance:** Renders at most `visibleCount + 2 * overscan` items. For a 600px container with 40px items: ~15 visible + 10 overscan = 25 DOM nodes regardless of list length.

---

## 11. Component Dependency Graph

```
Foundational (09)           Expressive (this doc)
─────────────               ──────────────────────
Panel ─────────────────────→ ResizablePane → PaneGrid, PaneGroup
Surface, Skeleton ─────────→ LoadingTransition, LazyPane, ConnectionGuard
StepProgress (base) ───────→ StepProgress (enhanced), GradientStepRail
                             VerticalTimeline, MilestoneProgress
                             CircularProgress
ProgressBar ───────────────→ MilestoneProgress (fill bar)
AnimatedNumber ────────────→ AgentMetricBar, CircularProgress, InferenceFeed
Badge, Pulse ──────────────→ AgentContainer, AgentFeed, EventStream
Sparkline ─────────────────→ InferenceFeed (inline sparkline)
ScrollArea ────────────────→ VerticalTimeline, AgentFeed, EventStream, FloatingChat
Transition ────────────────→ ContentSwitch, LazyPane, ConnectionGuard, Modal, Drawer
Stack, Grid ───────────────→ AgentMetricBar, MetricGrid layouts
Tabs ──────────────────────→ EventStream filter bar
Input ─────────────────────→ FloatingChat, CommandPalette
Chip ──────────────────────→ EventStream filters, CommandPalette categories
Card ──────────────────────→ VerticalTimeline step cards
Toast ─────────────────────→ BlockFeed (copy confirmation)

WebGL primitives (independent, no 09 deps):
  ParticleField, NoiseBackground, FluidGradient, HeartbeatLine, GlitchOverlay

Composed:
  AmbientContainer = Panel + (NoiseBackground | ParticleField | FluidGradient)
  AgentContainer = Panel + AgentHeartbeat + AgentMetricBar + AgentFeed + AmbientContainer
  AgentAvatar = SVG (standalone, no deps)
```

---

## 12. File Structure (Additions to 09)

```
src/primitives/
├── panes/
│   ├── ResizablePane.tsx
│   ├── ResizablePane.css
│   ├── PaneGrid.tsx
│   ├── PaneGroup.tsx
│   └── PaneGroup.css
│
├── transitions/
│   ├── LoadingTransition.tsx       — Pixelated dither transition
│   ├── ContentSwitch.tsx           — Animated content replacement
│   ├── LazyPane.tsx                — Connection-aware pane
│   └── ConnectionGuard.tsx         — Connection gate wrapper
│
├── progress/
│   ├── StepProgress.tsx            — Enhanced (extends 09 base)
│   ├── StepProgress.css
│   ├── GradientStepRail.tsx
│   ├── GradientStepRail.css
│   ├── VerticalTimeline.tsx
│   ├── VerticalTimeline.css
│   ├── CircularProgress.tsx
│   └── MilestoneProgress.tsx
│
├── webgl/
│   ├── ParticleField.tsx           — Generalized particle background
│   ├── NoiseBackground.tsx         — Shader noise
│   ├── FluidGradient.tsx           — Gradient mesh
│   ├── HeartbeatLine.tsx           — EKG waveform
│   ├── GlitchOverlay.tsx           — Visual corruption
│   ├── AmbientContainer.tsx        — Convenience wrapper
│   └── shaders/
│       ├── noise.frag              — Simplex/Perlin/Worley noise GLSL
│       └── fluid.frag              — Gradient mesh interpolation GLSL
│
├── agent/
│   ├── AgentContainer.tsx
│   ├── AgentContainer.css
│   ├── AgentMetricBar.tsx
│   ├── AgentMetricBar.css
│   ├── AgentFeed.tsx
│   ├── AgentFeed.css
│   ├── AgentHeartbeat.tsx
│   ├── AgentHeartbeat.css
│   ├── AgentAvatar.tsx
│   └── AgentAvatar.css
│
├── feeds/
│   ├── EventStream.tsx
│   ├── EventStream.css
│   ├── BlockFeed.tsx
│   ├── BlockFeed.css
│   ├── InferenceFeed.tsx
│   └── InferenceFeed.css
│
├── chat/
│   ├── FloatingChat.tsx
│   ├── FloatingChat.css
│   ├── ChatBubble.tsx
│   └── ChatBubble.css
│
├── overlays/
│   ├── Modal.tsx                   — Enhanced from 09
│   ├── Modal.css
│   ├── Drawer.tsx                  — Enhanced from 09
│   ├── Drawer.css
│   ├── CommandPalette.tsx
│   └── CommandPalette.css
│
├── layout/
│   ├── StickyTopLayout.tsx
│   ├── MasonryGrid.tsx
│   ├── MasonryGrid.css
│   ├── TreeView.tsx
│   ├── TreeView.css
│   ├── VirtualList.tsx
│   └── VirtualList.css
│
└── index.ts                        — Updated barrel exports
```

---

## 13. Implementation Priority

### Wave 2 (alongside 09's Wave 2 — data display)

| # | Component | Why now | Effort |
|---|-----------|---------|--------|
| 1 | `StepProgress` (enhanced) | Every page shows pipeline phases | M |
| 2 | `CircularProgress` | Bench metrics, pipeline progress | S |
| 3 | `VerticalTimeline` | Plan execution detail, event history | M |
| 4 | `MilestoneProgress` | Pipeline progress bars | S |
| 5 | `VirtualList` | Event feeds, episode lists already hitting 1000+ items | M |
| 6 | `StickyTopLayout` | Fixes the body overflow hidden bug now | S |
| 7 | `TreeView` | Plan task hierarchies in Orchestrate page | M |

### Wave 3 (alongside 09's Wave 3 — interactive + feedback)

| # | Component | Why now | Effort |
|---|-----------|---------|--------|
| 8 | `Modal` (enhanced) | Needed for detail views, confirmations | S |
| 9 | `Drawer` (enhanced) | Agent detail panels, settings | S |
| 10 | `CommandPalette` | Quick navigation, keyboard power users | M |
| 11 | `LoadingTransition` | Polish: replace blank->content jarring cuts | M |
| 12 | `ContentSwitch` | Polish: tab content transitions | S |
| 13 | `LazyPane` | Connection-aware loading for all real-time panels | M |
| 14 | `ConnectionGuard` | Startup experience when server not running | S |
| 15 | `GradientStepRail` | Visual polish for hero pipeline display | M |
| 16 | `EventStream` | Universal feed component for all real-time data | L |
| 17 | `InferenceFeed` | Observe page LLM activity | M |
| 18 | `FloatingChat` | Agent interaction from any page | L |
| 19 | `ChatBubble` | Trigger for FloatingChat | S |

### Wave 4 (alongside 09's Wave 4 — page-by-page migration + polish)

| # | Component | Why now | Effort |
|---|-----------|---------|--------|
| 20 | `ResizablePane` | Pane management for power users | L |
| 21 | `PaneGrid` | Full workbench layout | L |
| 22 | `PaneGroup` | Simpler split layouts | M |
| 23 | `AgentContainer` | Multi-agent page with per-agent scoping | L |
| 24 | `AgentMetricBar` | Per-agent metrics in containers | M |
| 25 | `AgentFeed` | Per-agent event feeds | M |
| 26 | `AgentHeartbeat` | Visual agent identity | M |
| 27 | `AgentAvatar` | Procedural agent representation | L |
| 28 | `ParticleField` | Generalize existing HeroParticleField | M |
| 29 | `NoiseBackground` | Ambient expression for containers | M |
| 30 | `FluidGradient` | Premium ambient effect | M |
| 31 | `HeartbeatLine` | Agent containers decoration | S |
| 32 | `GlitchOverlay` | Error/stress expression | S |
| 33 | `AmbientContainer` | Convenience wrapper | S |
| 34 | `MasonryGrid` | Knowledge page, research results | M |
| 35 | `BlockFeed` | Chain integration (Phase 2+, defer) | M |

**Effort key:** S = <1 day, M = 1-2 days, L = 3-5 days.

**Total estimate:** ~45-65 dev days for all 35 components. Wave 2 alone (7 components): ~8-10 days.

---

## 14. Performance Contracts

| Category | Contract |
|----------|----------|
| WebGL components | 30fps cap. `requestAnimationFrame` with frame skip. Canvas `pointer-events: none` |
| Virtualized lists | Max 25-30 DOM nodes regardless of data size |
| Animations | All CSS-based where possible. JS animations use `requestAnimationFrame`, never `setInterval` |
| Canvas rendering | `willReadFrequently: false` unless reading pixels (GlitchOverlay). DPR-aware via `useCanvasSetup` hook |
| Bundle size | WebGL shaders inlined as template literals, not separate fetch. Tree-shakeable exports |
| Reduced motion | Every component checks `prefers-reduced-motion`. WebGL degrades to static render or CSS gradient |
| Memory | Event feeds cap at `maxEvents`. Old entries removed from DOM. Scroll position preserved |
| Resize | All resize handlers debounced (100-250ms). `ResizeObserver`, not window resize event |
| State subscriptions | Components unsubscribe on unmount. No orphaned SSE/WS listeners |

---

## 15. Inference, Learning & Artifact Primitives

Components that visualize the inference pipeline, cascade router learning, and artifact accumulation. These are the primitives that make model routing, cost tracking, and knowledge crystallization *visible* in the UI. All confidence-driven visuals follow the progressive intensity system: 0.0 = ghost/dim, 0.5 = building, 0.8 = confident, 1.0 = crystallized with sparkle (cross-ref `04-DESIGN-SYSTEM.md` section 4 animation system, section 5 atmospheric layers).

**Tier color system** (used across all components in this section):

| Tier | Semantic | Color token | Hex |
|------|----------|-------------|-----|
| T0 | cheap/fast (haiku-class) | `--status-success` | `#4ade80` (green) |
| T1 | balanced (sonnet-class) | `--bone` | `#c8b890` (bone) |
| T2 | premium (opus-class) | `--rose-bright` | `#cc90a8` (rose) |

---

### 15.1 `<InferenceTag>` — Inference Annotation Pill

Compact inline annotation showing model, token counts, cost, and latency for any inference event. Used in trace views, event feeds, episode detail panels, and inline within agent output streams.

```tsx
interface InferenceTagProps {
  tier: 'T0' | 'T1' | 'T2';
  model: string;          // "haiku", "sonnet", "opus"
  provider?: string;      // "anthropic", "openai"
  inputTokens?: number;
  outputTokens?: number;
  cost?: number;           // dollar amount
  latencyMs?: number;
  compact?: boolean;       // Single-line vs expanded. Default false
  animate?: boolean;       // Slot machine animation on model change. Default false
  className?: string;
}
```

**Visual (expanded mode):**

```
┌──────────────────────────────────────────────────┐
│ ▸T1  Sonnet   →2.4k  ←890   $0.003   142ms      │
│  ▲    ▲        ▲      ▲      ▲        ▲         │
│  │    │        │      │      │        └ latency  │
│  │    │        │      │      └ cost (mono, dim)  │
│  │    │        │      └ output tokens            │
│  │    │        └ input tokens                    │
│  │    └ model name (mono, --text-primary)        │
│  └ tier badge, colored per tier table            │
└──────────────────────────────────────────────────┘
```

**Visual (compact mode):**

```
┌────────────────┐
│ ▸T1  Sonnet    │
└────────────────┘
```

**Tier badge:** 24px wide pill with 2px left border in tier color. Text: 10px mono uppercase. Background: tier color at 0.12 opacity. The `▸` chevron is tier-colored.

**Token counts:** Mono 10.5px. Arrow prefix: `→` for input (sending to model), `←` for output (receiving from model). Numbers use `AnimatedNumber` from `09-DESIGN-PRIMITIVES.md` for smooth transitions when values update mid-stream.

**Cost:** Mono 10px, `--text-dim`. Dollar sign prefix. Omitted if `cost` is undefined.

**Latency:** Mono 10px, `--text-ghost`. Suffix `ms`. Color escalates: <200ms = `--text-ghost`, 200-1000ms = `--text-dim`, >1000ms = `--warning`.

**Animation (when `animate=true`):** On model change, the model name characters roll vertically using `ModelSlot` (15.2) logic. Tier badge cross-fades color over 150ms `--ease-snappy`.

**Reduced motion:** No character roll; model name cross-fades. Tier badge instant color swap.

**Layout:** Inline-flex, `gap: var(--gap-xs)`. Wraps naturally in tight containers. Each segment is a separate `<span>` for independent styling.

---

### 15.2 `<ModelSlot>` — Animated Model Selector Display

Slot-machine animation when the cascade router selects a new model. Each character rolls vertically through intermediate values before settling, left-to-right with stagger. Used in `InferenceTag`, nav bar model indicator, and bench comparison headers.

```tsx
interface ModelSlotProps {
  model: string;           // e.g. "sonnet", "opus", "haiku"
  tier: 'T0' | 'T1' | 'T2';
  size?: 'sm' | 'md' | 'lg';   // Default 'md'
  className?: string;
}
```

**Visual:**

```
 ┌─────────────────────────┐
 │  s  o  n  n  e  t       │   ← settled state
 └─────────────────────────┘

      ↓ model changes to "opus" ↓

 ┌─────────────────────────┐
 │  ▒  ▒  ▒  ▒  ▒  ▒       │   ← all slots rolling
 │  ↕  ↕  ↕  ↕             │
 │  o  p  u  s              │   ← settling left-to-right
 └─────────────────────────┘
```

**Size tokens:**

| Size | Font size | Letter spacing | Height |
|------|-----------|----------------|--------|
| `sm` | 10px mono | 0.5px | 16px |
| `md` | 12px mono | 1px | 20px |
| `lg` | 16px mono | 1.5px | 28px |

**Animation mechanics:**

1. On `model` prop change, each character position gets a vertical scroll container
2. Container is `overflow: hidden`, height = one character line-height
3. Inside: a column of 5-8 random characters plus the target character at the bottom
4. Column translates upward to reveal target, using `--ease-expo` timing
5. Stagger: each position starts 40ms after the previous (left-to-right settle)
6. Total duration: `model.length * 40ms + 200ms` base
7. During roll, text color is `--text-ghost`; on settle, transitions to `--text-primary` over 100ms

**Tier underline:** 2px bottom border in tier color. Fades in 100ms after the last character settles. Glow: `0 2px 8px` in tier color at 0.2 opacity.

**Reduced motion:** Instant swap, no roll. Tier underline appears immediately.

---

### 15.3 `<CyberneticIntensity>` — Progressive Confidence Wrapper

Wraps any element with progressive visual intensity based on a 0-1 confidence value. This is the universal building block for the "progressive intensity" pattern referenced throughout the design system. Any component that needs to express certainty, health, or significance uses this wrapper.

```tsx
interface CyberneticIntensityProps {
  value: number;           // 0.0 - 1.0
  children: ReactNode;
  showLabel?: boolean;     // Show percentage label. Default false
  variant?: 'background' | 'border' | 'glow';  // Default 'background'
  className?: string;
}
```

**Progressive intensity mapping:**

| Value range | Visual state | Background variant | Border variant | Glow variant |
|-------------|-------------|-------------------|----------------|-------------|
| 0.0 - 0.2 | **Ghost** | `--bg-glass` at 0.3 opacity | `--border-soft` | No glow |
| 0.2 - 0.4 | **Faint** | `--bg-glass` at 0.5 opacity | `--border` | `0 0 4px` at 0.05 |
| 0.4 - 0.6 | **Building** | `--rose-deep` at 0.2 opacity | `--border-strong` | `0 0 8px` at 0.1 |
| 0.6 - 0.8 | **Confident** | `--rose-deep` at 0.4 opacity | `--rose-dim` | `0 0 12px` at 0.15 |
| 0.8 - 0.95 | **Crystallized** | `--rose-ember` at 0.5 opacity | `--rose` | `0 0 16px` at 0.2 |
| 0.95 - 1.0 | **Sparkle** | `--rose-ember` at 0.6 opacity + shimmer | `--rose-bright` + pulse | `0 0 20px` at 0.3 + sparkle particles |

**Visual (background variant at different values):**

```
  value=0.1          value=0.5          value=0.9          value=1.0
┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│  ░░░░░░░░░  │  │  ▒▒▒▒▒▒▒▒▒  │  │  ▓▓▓▓▓▓▓▓▓  │  │  ▓▓▓▓▓▓▓▓▓  │
│  ░ ghost ░  │  │  ▒building▒  │  │  ▓crystal▓  │  │  ▓sparkle▓✦ │
│  ░░░░░░░░░  │  │  ▒▒▒▒▒▒▒▒▒  │  │  ▓▓▓▓▓▓▓▓▓  │  │  ▓▓▓▓▓▓▓▓▓  │
└─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘
```

**Label:** When `showLabel=true`, a small mono pill (10px, `--text-dim`) appears top-right: `42%`. At >=0.8, label color transitions to `--text-primary`. At 1.0, label gets `value-flash` animation (cross-ref `04-DESIGN-SYSTEM.md` section 4.1).

**Sparkle effect (value >= 0.95):** 3-5 tiny particles (2px circles, `--rose-bright`) emanate from random edge positions. CSS `@keyframes`: translate outward 8-16px over 800ms, fade to 0. New particle every 400ms. Canvas-free: uses absolutely-positioned `<span>` elements, recycled to a pool of 6.

**Transition:** All visual properties interpolate smoothly. CSS custom property `--ci-value` set on the wrapper element, all visual mappings derived from it via `calc()` where possible. Fallback: `useEffect` with RAF for complex mappings. Transition duration: 300ms `--ease-out`.

**Reduced motion:** No sparkle particles. No shimmer. Background/border/glow still respond to value (static intensity).

---

### 15.4 `<ConfidenceMeter>` — Cascade Router Confidence

Horizontal confidence meter for the cascade router's learning state. Shows how confident the router is in its model selection for a given task category. Used in bench dashboards, learning inspection panels, and the TUI bridge.

```tsx
interface ConfidenceMeterProps {
  confidence: number;      // 0-1
  trend: 'improving' | 'stable' | 'declining';
  decisions: number;       // Total routing decisions made
  label?: string;
  className?: string;
}
```

**Visual:**

```
  ┌─ ROUTER CONFIDENCE ────────────── 847 decisions ──┐
  │                                                     │
  │  ┌──────────────────────────────────────────────┐  │
  │  │▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░░░░░░░░░░░░│  │
  │  └──────────────────────────────────────────────┘  │
  │       73%                                ▲ +2.1%   │
  │                                                     │
  └─────────────────────────────────────────────────────┘

  Legend:
  ▓ = filled region (progressive intensity from CyberneticIntensity)
  ░ = unfilled region (--bg-glass)
  ▲ = trend arrow (improving)
```

**Bar:** 6px tall, full width, `border-radius: 3px`. Unfilled region: `--bg-glass`. Filled region uses `CyberneticIntensity` (15.3) progressive intensity -- the fill itself gets more vivid as confidence increases. At <0.4, fill is `--rose-deep` at low opacity. At >0.8, fill is `--rose-bright` with glow.

**Fill animation:** On confidence change, the bar width animates over 400ms `--ease-expo`. The fill color also transitions to match the new intensity band.

**Trend indicator:**

| Trend | Icon | Color | Animation |
|-------|------|-------|-----------|
| `improving` | `▲` | `--status-success` | Gentle bob: translateY(-1px) and back, 2s ease infinite |
| `stable` | `▸` | `--text-dim` | None |
| `declining` | `▼` | `--warning` | None |

**Trend delta:** Mono 10px next to trend arrow. Shows percentage change since last update (e.g., `+2.1%`, `-0.8%`). Uses `value-flash` animation (cross-ref `04-DESIGN-SYSTEM.md` section 4.1) on change.

**Decisions count:** Mono 10px, `--text-ghost`, right-aligned in the header. Format: `{n} decisions`. Uses `AnimatedNumber` for smooth count-up.

**Crystallization at >0.8:** When confidence exceeds 0.8, the entire meter wrapper gets `CyberneticIntensity` glow variant at the confidence value. The label text transitions from `--text-dim` to `--text-primary`. At >0.95, sparkle particles appear along the filled bar edge.

**Reduced motion:** No trend bob. No sparkle. Fill width snaps instantly. Colors still reflect confidence level.

---

### 15.5 `<TraceAnnotation>` — Inline Trace/Log Annotation Strip

Horizontal pill strip for annotating trace entries, log lines, and event feed items with agent identity, model tier, confidence, and cost. Designed to sit inline within `EventStream` (section 7) items or `AgentFeed` (section 6) entries.

```tsx
interface TraceAnnotationProps {
  agentName?: string;
  agentColor?: string;     // hex, overrides default agent color
  tier?: 'T0' | 'T1' | 'T2';
  model?: string;
  confidence?: number;     // 0-1, drives background intensity via CyberneticIntensity
  cost?: number;
  className?: string;
}
```

**Visual:**

```
  ┌──────────────────────────────────────────────────────┐
  │  ● planner   ▸T1 Sonnet   ░░░▒▒▓▓ 68%   $0.003    │
  │  ▲            ▲             ▲              ▲         │
  │  │            │             │              └ cost    │
  │  │            │             └ confidence bar (mini)  │
  │  │            └ InferenceTag (compact)               │
  │  └ agent dot + name                                  │
  └──────────────────────────────────────────────────────┘
```

**Layout:** Inline-flex, `gap: var(--gap-sm)`, `align-items: center`. Height: 22px. All segments optional -- the strip renders only what props are provided.

**Agent segment:** 6px circle in `agentColor` (or fallback to hashed color from agent name). Name in mono 10px, `--text-soft`. Truncated with ellipsis at 80px max-width.

**Tier/model segment:** Delegates to `InferenceTag` in compact mode (15.1). Occupies 60-80px.

**Confidence segment:** Mini horizontal bar, 40px wide, 4px tall. Fill driven by `CyberneticIntensity` (15.3) progressive intensity. Percentage label in mono 9px to the right. Omitted if `confidence` is undefined.

**Cost segment:** Mono 10px, `--text-ghost`. Dollar sign prefix. Omitted if `cost` is undefined.

**Background intensity:** The entire strip gets a subtle background tint driven by `confidence` via `CyberneticIntensity` background variant. At 0.0: transparent. At 1.0: `--rose-deep` at 0.15 opacity. This provides at-a-glance confidence scanning in long trace lists.

**Reduced motion:** No animation. Static renders throughout.

---

### 15.6 `<ArtifactGem>` — Collectible Artifact Indicator

Represents an episode, insight, HDC fingerprint, or knowledge entry as a collectible gem. Shape and color encode type and significance. Used in artifact trays, episode lists, knowledge panels, and inline within trace annotations.

```tsx
interface ArtifactGemProps {
  type: 'episode' | 'insight' | 'hdc' | 'knowledge';
  significance: number;    // 0-1, maps to intensity/sparkle via CyberneticIntensity
  label?: string;
  onClick?: () => void;
  animate?: boolean;       // Sparkle on mount. Default true
  className?: string;
}
```

**Shape mapping:**

```
  episode       insight       hdc           knowledge
  (hexagon)     (diamond)     (circle)      (square)

    ╱╲            ◇            ●             ■
   ╱  ╲          ╱ ╲
  │    │        ◇   ◇
   ╲  ╱          ╲ ╱
    ╲╱            ◇
```

**Type visual spec:**

| Type | Shape | Base color | CSS shape method | Size (default) |
|------|-------|------------|-----------------|----------------|
| `episode` | Hexagon | `--rose` | `clip-path: polygon(25% 0%, 75% 0%, 100% 50%, 75% 100%, 25% 100%, 0% 50%)` | 20px |
| `insight` | Diamond | `--bone` | `clip-path: polygon(50% 0%, 100% 50%, 50% 100%, 0% 50%)` | 18px |
| `hdc` | Circle | `--dream` | `border-radius: 50%` | 16px |
| `knowledge` | Square | `--status-active` (teal) | `border-radius: 3px` | 16px |

**Progressive intensity (via CyberneticIntensity system):**

| Significance | Visual |
|-------------|--------|
| 0.0 - 0.3 | Ghost: base color at 0.2 opacity, no glow, `--text-ghost` label |
| 0.3 - 0.6 | Forming: base color at 0.5 opacity, faint inner glow, `--text-dim` label |
| 0.6 - 0.8 | Solid: base color at 0.8 opacity, visible glow (`0 0 8px` at 0.15), `--text-soft` label |
| 0.8 - 1.0 | Brilliant: base color at 1.0, strong glow (`0 0 12px` at 0.3), sparkle particles, `--text-primary` label |

**Sparkle animation (on mount, when `animate=true`):**

1. Gem scales from 0.6 to 1.0 over 200ms `--ease-snappy` (cross-ref `04-DESIGN-SYSTEM.md` section 4.1 `gate-pass` keyframes)
2. 4-6 sparkle particles burst outward from center, 12-20px travel, 600ms fade-out
3. Particles: 2px circles in the gem's base color at 0.6 opacity
4. After settle: if significance >= 0.8, 1-2 ambient sparkle particles continue every 2s

**Hover:** `translateY(-2px)` + glow intensifies by 50% over 150ms. Cursor: pointer if `onClick` provided.

**Label:** Mono 9px, positioned below gem, center-aligned. Truncated at 60px with ellipsis. Color follows significance intensity mapping above.

**Reduced motion:** No scale entrance. No sparkle particles. Gem appears at full opacity instantly. Hover: no translate, glow change only.

---

### 15.7 `<CrystallizeTransition>` — Achievement Celebration Effect

Canvas overlay effect triggered on achievements: high C-factor scores, batch gate passes, artifact discoveries, confidence crystallization events. Provides a moment of visual reward without interrupting interaction.

```tsx
interface CrystallizeTransitionProps {
  active: boolean;
  intensity?: number;      // 0-1, controls particle count and glow strength. Default 0.7
  children: ReactNode;
  className?: string;
}
```

**Visual (when `active` transitions to true):**

```
  ┌─────────────────────────────────┐
  │          ✦  .  ✦                │  ← t=0ms: sparkle particles spawn
  │       .    ✦    .  ✦            │     from element center/edges
  │     ✦   ╔═══════════╗   .      │
  │    .     ║  content  ║    ✦     │  ← t=100ms: prismatic shimmer
  │     ✦    ╚═══════════╝   .     │     on element background
  │       .    ✦    .  ✦            │
  │          ✦  .  ✦                │  ← t=200ms: ring pulse outward
  │              ○                  │     from element center
  │            ○   ○                │
  │          ○       ○              │  ← t=800ms: ring fades
  │            ○   ○                │
  │              ○                  │
  │                                 │  ← t=1500ms: all effects faded
  └─────────────────────────────────┘
```

**Effect layers (sequential, overlapping):**

| Layer | Start | Duration | Effect |
|-------|-------|----------|--------|
| 1. Sparkle particles | 0ms | 800ms | 8-16 particles (scaled by `intensity`) emanate from element bounding box edges. Each: 2-3px circle, random color from `[--rose-bright, --bone-bright, --dream-bright]`. Travel: 20-60px outward. Fade: opacity 0.8 to 0 over lifetime. |
| 2. Prismatic shimmer | 100ms | 600ms | Element background gets a sweeping linear gradient highlight: `linear-gradient(120deg, transparent, rgba(255,255,255,0.08), transparent)`. The gradient translates left-to-right across the element over 600ms. |
| 3. Ring pulse | 200ms | 800ms | Circle ring (2px stroke, `--rose-glow`) expands from element center outward. Start radius: 50% of element width. End radius: 150% of element width. Opacity: 0.4 to 0. Scale uses `--ease-expo`. |
| 4. Glow surge | 0ms | 1200ms | Element `box-shadow` surges: `0 0 0px` to `0 0 24px rgba(220,165,189, 0.2 * intensity)` over 200ms, then decays back to 0 over remaining 1000ms. |

**Auto-deactivation:** After 1500ms, all effects have faded. Component resets internal state. If `active` is still true, does NOT re-trigger (one-shot). Re-trigger requires `active` going false then true again.

**Rendering:** Sparkle particles and ring pulse render on a `<canvas>` overlay positioned over the wrapped element. Canvas: `position: absolute`, `inset: -40px` (overflow for ring), `pointer-events: none`, `z-index: 10`. Uses `requestAnimationFrame`. Canvas is created on trigger, destroyed after fade-complete.

**Performance:** Particle count scales with `intensity`: at 0.3 = 4 particles, at 0.7 = 10, at 1.0 = 16. Canvas removed from DOM after animation completes. No persistent GPU cost.

**Reduced motion:** No particles, no ring. Only the glow surge (layer 4) plays, at 50% intensity and 200ms duration. Prismatic shimmer replaced with a single 100ms opacity flash.

---

### 15.8 `<ArtifactTray>` — Collectible Counter/Accumulator

Persistent counter strip showing collected artifacts across the session. Designed to sit in `TopNav` or sidebar. Provides at-a-glance accumulation feedback and opens a detail drawer on click.

```tsx
interface ArtifactTrayProps {
  episodes: number;
  insights: number;
  hdcEntries: number;
  knowledgeEntries: number;
  onOpen?: () => void;
  className?: string;
}
```

**Visual:**

```
  ┌─────────────────────────────────────────────────┐
  │  ⬡ 24    ◇ 8    ● 156    ■ 42                  │
  │  ▲       ▲      ▲        ▲                      │
  │  │       │      │        └ knowledge (teal sq)  │
  │  │       │      └ hdc (dream circle)            │
  │  │       └ insights (bone diamond)              │
  │  └ episodes (rose hexagon)                      │
  └─────────────────────────────────────────────────┘
```

**Layout:** Inline-flex, `gap: var(--gap-md)`, `align-items: center`. Height: 28px. Background: `--bg-glass`. Border: `--border-soft`. Padding: `4px 12px`. Border-radius: 14px (pill shape).

**Each artifact slot:**

```
  ┌──────────┐
  │  ⬡  24   │
  │  ▲   ▲   │
  │  │   └ count (mono 11px, --text-soft)
  │  └ ArtifactGem at size=12px, significance=0.6
  └──────────┘
```

- Gem icon: `ArtifactGem` (15.6) at 12px size, fixed significance of 0.6 (solid, not ghost)
- Count: `AnimatedNumber` (from `09-DESIGN-PRIMITIVES.md`), mono 11px, `--text-soft`
- Gap between gem and count: `var(--gap-xs)` (4px)

**New artifact animation:** When any count increments:

1. The corresponding `ArtifactGem` icon plays `gate-pass` keyframe (cross-ref `04-DESIGN-SYSTEM.md` section 4.1): scale 0.8 to 1.05 to 1.0, 200ms
2. Count number gets `value-flash` keyframe: bone highlight for 300ms
3. A single sparkle particle (2px, gem's base color) pops upward 8px and fades over 400ms
4. All three play simultaneously on each increment

**Batch arrival:** If multiple artifacts arrive within 200ms (common during gate pipeline completion), the tray plays a single combined animation: all affected gems pulse together, and the tray background briefly flashes to `--bg-glass-active` for 200ms.

**Hover:** Tray background transitions to `--bg-glass-hover` over 150ms. Cursor: pointer if `onOpen` provided. Tooltip (if no drawer): `"24 episodes, 8 insights, 156 HDC entries, 42 knowledge entries"`.

**Empty state:** Counts at 0 render with `--text-ghost` color. Gem icons render at significance 0.1 (ghost).

**Reduced motion:** No gem pulse. No sparkle particles. Count numbers update instantly (no `AnimatedNumber` roll). Background flash on batch arrival still plays (color transition only).

---

### 15.9 Component Dependency Graph (Section 15 Additions)

```
Foundational (09)           Section 15 (this section)
─────────────               ──────────────────────────
AnimatedNumber ────────────→ InferenceTag (token counts), ConfidenceMeter (decisions),
                             ArtifactTray (counts)
Badge ─────────────────────→ InferenceTag (tier badge)

Section 15 internal:
  CyberneticIntensity ─────→ ConfidenceMeter (fill + wrapper glow)
                      ─────→ TraceAnnotation (background + mini bar)
                      ─────→ ArtifactGem (significance mapping)
  InferenceTag ────────────→ TraceAnnotation (compact tier/model segment)
  ModelSlot ───────────────→ InferenceTag (animate=true model change)
  ArtifactGem ─────────────→ ArtifactTray (slot icons)
  CrystallizeTransition ───→ (standalone, wraps any element)

Consumed by existing components:
  EventStream (section 7) ──→ TraceAnnotation (per-event annotation strip)
  AgentFeed (section 6) ────→ TraceAnnotation, InferenceTag
  TopNav ───────────────────→ ArtifactTray, ModelSlot
```

---

### 15.10 File Structure (Section 15 Additions)

```
src/primitives/
├── inference/
│   ├── InferenceTag.tsx
│   ├── InferenceTag.css
│   ├── ModelSlot.tsx
│   ├── ModelSlot.css
│   ├── ConfidenceMeter.tsx
│   ├── ConfidenceMeter.css
│   ├── TraceAnnotation.tsx
│   └── TraceAnnotation.css
│
├── intensity/
│   ├── CyberneticIntensity.tsx
│   ├── CyberneticIntensity.css
│   ├── CrystallizeTransition.tsx
│   └── CrystallizeTransition.css
│
├── artifacts/
│   ├── ArtifactGem.tsx
│   ├── ArtifactGem.css
│   ├── ArtifactTray.tsx
│   └── ArtifactTray.css
│
└── index.ts                        — Updated barrel exports
```

---

### 15.11 Implementation Priority (Section 15)

| # | Component | Why | Effort | Wave |
|---|-----------|-----|--------|------|
| 1 | `CyberneticIntensity` | Foundation for all confidence-driven visuals; blocks 4 other components | M | Wave 2 |
| 2 | `InferenceTag` | Used everywhere inference events appear; the core annotation pill | S | Wave 2 |
| 3 | `ModelSlot` | Small, self-contained; enhances InferenceTag and nav | S | Wave 2 |
| 4 | `ArtifactGem` | Blocks ArtifactTray; small, self-contained | S | Wave 3 |
| 5 | `ConfidenceMeter` | Bench and learning dashboards need this for router visibility | M | Wave 3 |
| 6 | `TraceAnnotation` | EventStream and AgentFeed integration; depends on InferenceTag + CyberneticIntensity | M | Wave 3 |
| 7 | `ArtifactTray` | TopNav integration; depends on ArtifactGem | S | Wave 3 |
| 8 | `CrystallizeTransition` | Polish/reward layer; standalone, can defer | M | Wave 4 |

**Effort key:** S = <1 day, M = 1-2 days, L = 3-5 days.

**Total estimate:** ~8-12 dev days for all 8 components. Wave 2 (3 components): ~3-4 days.

---

### 15.12 Performance Contracts (Section 15)

| Category | Contract |
|----------|----------|
| CyberneticIntensity | CSS custom properties + `calc()` for intensity mapping. No JS per-frame. Sparkle particles: max 6 DOM nodes recycled |
| ModelSlot | Overflow-hidden divs with CSS transforms. No canvas. Max model.length DOM nodes for slots |
| CrystallizeTransition | Canvas created on trigger, destroyed after 1500ms. Max 16 particles. No persistent GPU cost |
| ArtifactGem | CSS `clip-path` for shapes. No SVG, no canvas. Sparkle: max 2 ambient particles (absolutely-positioned spans) |
| ArtifactTray | Debounce batch arrivals within 200ms window. Single combined animation for batches |
| ConfidenceMeter | Bar width via CSS `transform: scaleX()` (GPU-composited). No layout thrash on confidence updates |
| Reduced motion | All components check `prefers-reduced-motion`. Particles/rolls disabled. Color/opacity transitions preserved |

---

## 16. Multi-Agent Coordination

Components for visualizing agent-to-agent handoffs, coordination flows, and multi-agent pipelines. These express the multi-agent model that is Roko's core differentiator -- when agents delegate, collaborate, or transfer artifacts, these primitives make the flow visible.

**Layout context:** Handoff visualizations sit inline within scenario panels, task boards, and agent containers. They participate in the normal page scroll. The flow line between agents uses CSS animations for crystal particles, not canvas.

### 16.1 `<AgentHandoff>` — Agent Coordination Flow

Visualizes the handoff of work, artifacts, or context between two agents. Renders as two agent identity nodes connected by an animated flow line with directional crystal particles.

**File:** `src/components/agent/AgentHandoff.tsx` + `AgentHandoff.css`

```tsx
interface AgentInfo {
  name: string;
  role?: string;
  status?: 'idle' | 'working' | 'done';
}

interface AgentHandoffProps {
  from: AgentInfo;
  to: AgentInfo;
  status: 'pending' | 'active' | 'done' | 'error';
  direction?: 'forward' | 'reverse' | 'bidirectional';
  label?: string;           // Flow description, e.g., "PRD Draft"
  sublabel?: string;        // Secondary info, e.g., "3 artifacts  $0.02"
  artifacts?: number;       // Artifact count badge on the flow line
  progress?: number;        // 0-1, optional progress indicator on the flow line
  compact?: boolean;        // Smaller variant for inline use
  className?: string;
}
```

**Visual spec:**

```
  ┌──────────┐                              ┌──────────┐
  │  ◉ ALPHA │  ◇═══◇═══◇═══▶ PRD Draft   │  ◉ BETA  │
  │  writer  │  3 artifacts  $0.02         │  reviewer │
  │  working │                              │  idle    │
  └──────────┘                              └──────────┘
```

Compact variant (single-line, smaller agent nodes):

```
  ◉ alpha  ──◇◇◇──▶  ◉ beta     PRD Draft (3 artifacts)
```

**Agent nodes:**

Each node renders an `AgentAvatar` (from `components/agent/AgentAvatar.tsx`) with the agent's name-hashed color, plus a text label block showing name (mono 11px uppercase), role (mono 9px, `--text-dim`), and status (mono 9px, status-colored). Default size: 48px avatar, 36px in compact mode.

**Flow line and status states:**

| Status | Line style | Particles | Glow | Color |
|--------|-----------|-----------|------|-------|
| `pending` | 2px dashed, `--border` | None | None | `--text-ghost` |
| `active` | 2px solid, `--rose-dim` | 5 crystal particles flowing per direction | `0 0 12px rgba(216, 154, 178, 0.4)` on line | `--rose-bright` |
| `done` | 2px solid, `--success` | None (settled) | `0 0 8px rgba(122, 138, 120, 0.3)` residual | `--success` |
| `error` | 2px dashed, `--status-error` | None | Red pulse on line | `--status-error` |

**Crystal particles (active state):**

5 diamond-shaped particles (6px, rotated 45deg squares) per direction, flowing along the line path. CSS animation: `translateX(0%) -> translateX(100%)` with staggered delays (each particle offset by `flowDuration / particleCount`). Flow duration: 2s. Particle color: `--rose-bright` with varying opacity (0.4-0.8). Particles have a subtle glow trail.

**Direction modes:**

| Direction | Behavior |
|-----------|----------|
| `forward` | Particles flow left-to-right only. Arrow at right end. |
| `reverse` | Particles flow right-to-left only. Arrow at left end. |
| `bidirectional` | Particles flow both directions simultaneously. Arrows at both ends. |

**Artifacts badge:** When `artifacts` is set, a small pill appears centered on the flow line: `"3 artifacts"` in mono 9px, `--text-soft`, with `--bg-glass` background.

**Progress indicator:** When `progress` is set, the flow line has a gradient fill from left to right, filling to `progress * 100%` width. The unfilled portion remains dashed in pending style.

**Reduced motion:** No particle animation. Active state shows a solid line with gradient instead of particles. Done state shows a settled glow.

---

## 17. Gate Verification Primitives

Components for visualizing the gate verification pipeline -- the multi-step validation that runs after each agent task (compile, test, clippy, diff, etc.). These replace the basic pass/fail dots with expressive status cards that show running state, timing, and per-gate detail.

### 17.1 `<GateVerdictTicker>` — Gate Verdict Chip Strip

Horizontal strip of gate verdict chips, grouped by task. Shows real-time gate pipeline progress with pass/fail status, timing, and task grouping. Designed for scenario sidebars and task detail panels.

**File:** `src/components/GateVerdictTicker.tsx` + `GateVerdictTicker.css`

```tsx
interface GateVerdictItem {
  taskId: string;
  gate: string;            // "compile", "test", "clippy", "diff"
  passed: boolean;
  message?: string;        // Detail message on hover/tooltip
  durationMs: number;
}

interface GateVerdictTickerProps {
  verdicts: GateVerdictItem[];
  currentTaskId?: string;  // Highlights this task's gates, dims others
}
```

**Visual spec:**

```
  ┌─ COMPILE ─┐──┌─── TEST ───┐──┌── CLIPPY ──┐
  │  ✓  42ms  │  │  ✓  1.2s   │  │  ✓  380ms  │
  └───────────┘  └────────────┘  └────────────┘
```

With task grouping (multiple tasks):

```
  a1b2c3d4  ✓compile 42ms  ✓test 1.2s  |  e5f6g7h8  ✓compile 38ms  ✗test 2.1s
  ────────  ───────────────────────────    ────────  ─────────────────────────────
  (dimmed — previous task)                  (highlighted — current task)
```

**Per-chip rendering:**

| Element | Typography | Color |
|---------|-----------|-------|
| Pass icon (`✓`) | Mono 11px | `var(--status-success)` |
| Fail icon (`✗`) | Mono 11px | `var(--status-error)` |
| Gate name | Mono 10px uppercase | `var(--text-soft)` (current task), `var(--text-ghost)` (dimmed) |
| Duration | Mono 9px tabular-nums | `var(--text-dim)` |
| Task ID divider | Mono 9px | `var(--text-ghost)` |
| Task ID label | Mono 9px | `var(--text-ghost)`, truncated to 8 chars |

**Chip status variants:**

| Status | Background | Border | Icon | Animation |
|--------|-----------|--------|------|-----------|
| Pass | `rgba(122, 138, 120, 0.12)` | `rgba(122, 138, 120, 0.3)` | `✓` green | Brief scale-up 0.95->1.05->1.0 (150ms) on first render |
| Fail | `rgba(212, 138, 110, 0.12)` | `rgba(212, 138, 110, 0.3)` | `✗` red | Red pulse on border (400ms one-shot) |
| Dimmed | `transparent` | `var(--border-soft)` | Inherited | `opacity: 0.5` |

**Task grouping:** Verdicts are grouped by `taskId`, preserving insertion order. Groups are separated by a `|` divider character in `--text-ghost`. Each group is prefixed with the first 8 characters of the task ID.

**Current task highlighting:** Gates for `currentTaskId` render at full opacity. Gates for other tasks render at 0.5 opacity with muted colors. This provides temporal context -- you see the history of previous task gates while focusing on the current one.

**Layout:** Inline-flex, horizontal scroll if overflow, `gap: 6px` between chips within a group, `gap: 12px` between groups. Height: 28px.

**Tooltip:** Each chip shows a tooltip on hover with the gate name, full task ID, pass/fail status, and optional `message`.

**Reduced motion:** No scale-up or pulse on chip appearance. Static rendering.

### 17.2 Gate Verdict Card (Future)

A richer alternative to `GateVerdictTicker` for detail panels and full-page gate views. Includes running state with braille spinner, animated connectors, and all-pass celebration sweep. This is specified but not yet built -- the `GateVerdictTicker` covers the primary use case.

**Planned visual spec:**

```
  ┌─ COMPILE ─┐──┌─── TEST ───┐──┌── CLIPPY ──┐
  │  ⌈✓⌋ 42ms │  │  ⌈✓⌋ 1.2s  │  │  ⠹ running │
  └────────────┘  └────────────┘  └────────────┘
```

**Planned status states:**

| Status | Icon | Frame | Animation |
|--------|------|-------|-----------|
| Running | Braille spinner (`⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`) | `⌈ ⌋` bracket frame | Marching-ants border, spinner cycles at 10fps |
| Pass | `✓` | `⌈✓⌋` | Green flash (0.15s), then settled glow |
| Fail | `✕` | `⌈✕⌋` | Red pulse (0.4s one-shot) |
| Pending | `○` | `⌈○⌋` | Ghost opacity, no animation |
| Skipped | `—` | `⌈—⌋` | Struck-through text |

**Planned connectors:** Solid green line after pass, red line after first fail, marching-ants animated line for running.

**Planned all-pass celebration:** When all gates pass, a green scan line sweeps left-to-right across all cards (200ms), followed by a settled green glow on all connectors.

---

## 18. Terminal Enhancements

Configuration and styling for the xterm.js terminal integration. These are not standalone components but rather the terminal configuration decisions that make the embedded terminals feel native to the ROSEDUST design system.

### 18.1 xterm.js Configuration

Full terminal option set applied at `Terminal` construction in `src/hooks/useTerminal.ts`:

| Option | Value | Rationale |
|--------|-------|-----------|
| `fontSize` | `12` | Denser than default 14px; fits more output in pane |
| `fontFamily` | `'JetBrainsMono Nerd Font Mono', 'JetBrains Mono', 'SF Mono', monospace` | Nerd Font for powerline glyphs, fallback chain |
| `lineHeight` | `1.1` | Tight line spacing (default 1.2 wastes vertical space) |
| `letterSpacing` | `0` | No extra spacing; mono font handles this |
| `cursorStyle` | `'bar'` | Thin bar cursor (modern terminal feel vs block) |
| `cursorWidth` | `2` | 2px bar width, visible but not heavy |
| `cursorBlink` | `true` | Standard terminal behavior |
| `cursorInactiveStyle` | `'outline'` | Hollow rectangle when pane unfocused |
| `scrollback` | `5000` | Generous scrollback for reviewing agent output |
| `smoothScrollDuration` | `80` | Subtle smooth scroll (not jarring instant jumps) |
| `drawBoldTextInBrightColors` | `false` | Prevents bold text from being unreadably bright |
| `fontWeight` | `'400'` | Normal weight body text |
| `fontWeightBold` | `'600'` | Semi-bold for bold sequences |
| `minimumContrastRatio` | `1` | Disabled (trust theme colors, don't auto-correct) |
| `allowProposedApi` | `true` | Enable experimental xterm APIs for future use |
| `overviewRulerWidth` | `8` | Thin overview ruler on right edge (scrollbar minimap) |
| `customGlyphs` | `true` | Use xterm's built-in box-drawing/powerline glyph renderer |

### 18.2 ROSEDUST Terminal Theme

Full 16-color ANSI palette plus cursor and selection colors. Defined in `src/lib/rosedust-theme.ts`:

```typescript
const rosedustTheme: ITheme = {
  // Base
  background: '#0c0a10',                                    // Darker than --bg-void for depth
  foreground: '#c4b4c4',                                    // Matches --text-soft
  cursor: '#d89ab2',                                        // Rose-spectrum cursor
  cursorAccent: '#0c0a10',                                  // Matches background for bar cursor fill

  // Selection
  selectionBackground: 'rgba(184, 122, 148, 0.28)',         // Rose-tinted selection
  selectionForeground: '#f0e4d0',                           // Bright foreground in selection
  selectionInactiveBackground: 'rgba(68, 56, 68, 0.4)',     // Muted when terminal unfocused

  // Standard ANSI colors (ROSEDUST palette mapping)
  black: '#18141e',             // Deep void
  red: '#d48a6e',               // Warm terracotta (not harsh)
  green: '#8a9c86',             // Sage green (--success adjacent)
  yellow: '#d8a878',            // Warm amber (--warning adjacent)
  blue: '#8888a8',              // Muted lavender (--dream adjacent)
  magenta: '#d89ab2',           // Rose (matches cursor)
  cyan: '#6a9ea0',              // Teal (--status-active adjacent)
  white: '#e4d8b0',             // Warm bone (--bone-bright)

  // Bright ANSI colors
  brightBlack: '#443844',       // Rose-tinted grey
  brightRed: '#e8a088',         // Lighter terracotta
  brightGreen: '#a4c4a0',       // Brighter sage
  brightYellow: '#e8c090',      // Lighter amber
  brightBlue: '#a4a4c8',        // Lighter lavender
  brightMagenta: '#e8b5ce',     // Lighter rose
  brightCyan: '#8abcbe',        // Lighter teal
  brightWhite: '#f0e4d0',       // Cream (--text-strong adjacent)
};
```

**Color rationale:** The ANSI palette avoids harsh primaries. Every color is filtered through the ROSEDUST warm/muted spectrum. Reds are terracotta, greens are sage, blues are lavender. This ensures terminal output (including colored `cargo test` results, `git diff` output, and agent ANSI escape sequences) blends with the surrounding UI rather than creating jarring contrast.

### 18.3 Terminal CSS Polish

Styling overrides applied via `src/components/Terminal/TerminalPane.css`:

**Scrollbar:**
```css
.xterm-viewport::-webkit-scrollbar {
  width: 6px;                           /* Thin, not standard 12px */
}
.xterm-viewport::-webkit-scrollbar-thumb {
  background: rgba(168, 112, 140, 0.3);  /* Rose-tinted thumb */
  border-radius: 3px;
}
.xterm-viewport::-webkit-scrollbar-thumb:hover {
  background: rgba(168, 112, 140, 0.5);  /* Brighter on hover */
}
.xterm-viewport::-webkit-scrollbar-track {
  background: transparent;
}
```

**Focus glow:**
```css
.terminal-pane:focus-within {
  box-shadow: 0 0 0 1px rgba(220, 165, 189, 0.2);  /* Subtle rose ring */
}
```

**Header bar density (in `src/pages/Demo.css`):**

| Property | Value | Change from default |
|----------|-------|-------------------|
| Header padding | `2px 10px` | Was `6px 12px` |
| Label font size | `10px` mono | Was `11px` |
| Status font size | `9px` mono | Was `10px` |
| Status dot size | `5px` | Was `6px` |

### 18.4 Bottom-Anchored Text Approach

Terminal output is bottom-anchored: when the terminal has few lines of output, text appears at the bottom of the pane rather than the top. This is achieved by writing newlines to fill the visible rows before the first real output:

```typescript
// In useTerminal.ts, after terminal.open():
const fillRows = term.rows - 1;  // Leave one row for the first line
term.write('\r\n'.repeat(Math.max(0, fillRows)));
```

This creates the visual effect of output "arriving from below" rather than a blank terminal with text stuck to the top edge. Combined with smooth scroll, new output lines appear to flow upward naturally.
