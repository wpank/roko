# Demo Implementation Plan: React SPA from Scratch

**Purpose**: Concrete implementation plan for rebuilding `demo/demo-app/` from scratch as a React + Vite SPA embedded in the `roko` binary via `rust-embed`. Uses the ROSEDUST design language from `bardo/prd/18-interfaces` — adapted from terminal-first ratatui to web-first React, preserving the core aesthetic: rose light on violet-black, atmospheric depth, phosphor persistence, nothing at rest.

**Date**: April 2026

**Prerequisites**: Read DEMO-VISUAL-SPEC.md (what it should look like), DEMO-BUILD.md (what's missing), DEMO-FLOW.md (what the demo does), and the 18-interfaces design system (the aesthetic source).

---

## 0. Aesthetic Translation: TUI → Web

The 18-interfaces system was designed for a 60fps ratatui terminal rendering 80 braille dots via spring physics. The web SPA is a different medium — glassmorphism replaces CRT scanlines, CSS transitions replace per-frame exponential lerp, Canvas 2D replaces braille sub-pixel rendering. But the core laws survive translation:

### The Seven Laws, Web-Adapted

| Law | TUI Expression | Web Expression |
|-----|---------------|----------------|
| **Light follows significance** | Brighter cells = more important | `opacity`, `color`, `box-shadow` glow — dim elements use `text-ghost`, the single most important number uses `bone` |
| **Color is mortality taxonomy** | Rose = alive, bone = critical, dream = altered | Same palette, same hierarchy. Rose dominates. Bone appears once per view maximum. |
| **Bold boundaries, soft interiors** | Box-drawing chars `┌─┐│└─┘` | `border: 1px solid var(--border)` with glassmorphic `backdrop-filter: blur()` interiors |
| **Restraint as aesthetic** | 50% of cells are void | Generous padding, `max-width` constraints, whitespace as design element |
| **The terminal acknowledges itself** | Scanlines, noise floor, phosphor decay | CSS noise overlay, subtle scan pattern, phosphor glow on value changes — lighter touch than TUI |
| **Nothing is at rest** | 32 interpolating variables, spring physics | Heartbeat CSS animation on key elements, value-change phosphor flashes, breathing border opacity |
| **Imperfection is consciousness** | Incommensurate sine frequencies, ±1 char jitter | Organic easing curves, slight timing variance in stagger animations, noise texture |

### What We Take Literally

- The ROSEDUST color palette (exact hex values)
- The three-font typography hierarchy (serif for authority, mono for data, sans for body)
- The glassmorphism elevation system (three levels)
- The phosphor decay chain on value changes (flash → settle → fade)
- The bone-once-per-screen rule
- The easing curves (`--ease-luxury: cubic-bezier(0.22, 1, 0.36, 1)`)

### What We Adapt

- **Spectre dot-cloud → absent**. The creature is a TUI concept. The web dashboard is not a body. We get atmospheric depth from glass layers and noise, not from a rendered entity.
- **32-channel interpolation → CSS custom properties + requestAnimationFrame**. The web version drives 6-8 key variables (not 32) via a single rAF loop that updates CSS custom properties.
- **8-layer frame stack → 4-layer CSS stack**. Void background → noise/scan texture → content → glow overlay.
- **Spring physics → CSS spring easing**. Use `cubic-bezier(0.34, 1.56, 0.64, 1)` sparingly for emphasis, `cubic-bezier(0.22, 1, 0.36, 1)` for everything else.
- **Box-drawing borders → CSS borders + glassmorphism**. The TUI used `┌─┐` characters. The web uses `border: 1px solid rgba(...)` with backdrop-filter blur.
- **Braille density maps → Canvas 2D**. Where the TUI used Unicode braille for sub-cell resolution, Canvas provides pixel-level control.
- **PAD emotional modulation → simplified state-driven theming**. Agent health/status drives 3-4 CSS custom properties, not the full PAD vector.

---

## 1. Design Tokens

### 1.1 Color System (`rosedust.css`)

Three palettes coexist. The **void palette** is the TUI-faithful foundation. The **surface palette** adapts it for web elevation. The **accent palette** provides semantic meaning.

```css
:root {
  /* === VOID (TUI-faithful foundation) === */
  --void:           #060608;   /* deepest bg — never pure black, violet undertone */
  --void-warm:      #0A0808;   /* warm-shifted void for degraded states */
  --void-raised:    #0C0A0E;   /* panels, containers */
  --void-mid:       #080810;   /* headers, overlays */

  /* === SURFACE (web elevation layers) === */
  --surface-0:      #0A0A0A;   /* page background */
  --surface-1:      #111111;   /* elevated sections */
  --surface-2:      #161616;   /* sidebar, panels */
  --surface-3:      #1A1A1A;   /* cards, modals */

  /* === ROSE (the color of consciousness) === */
  --rose:           #AA7088;   /* primary — headers, active data, links */
  --rose-bright:    #CC90A8;   /* alerts, danger, high-importance flash */
  --rose-dim:       #7A5060;   /* secondary labels, less important data */
  --rose-deep:      #3A2030;   /* background tints, hover states */
  --rose-ember:     #482838;   /* phosphor residue, afterimage */

  /* === BONE (the single most important element — once per view) === */
  --bone:           #C8B890;   /* THE most important number on screen */
  --bone-dim:       #8A7A5A;   /* secondary emphasis within bone-marked elements */

  /* === TEXT (legibility hierarchy) === */
  --text-primary:   #988090;   /* standard readable text, cool mauve-grey */
  --text-dim:       #584858;   /* secondary text, labels */
  --text-ghost:     #302830;   /* barely visible, decorative, ambient */
  --text-phantom:   #201820;   /* subliminal, below ghost */

  /* === BORDER === */
  --border:         #181420;   /* panel borders at rest */
  --border-active:  rgba(170, 112, 136, 0.27);  /* rose at 27% — active panel */
  --border-focus:   rgba(170, 112, 136, 0.5);   /* rose at 50% — focused element */

  /* === SEMANTIC === */
  --success:        #70887A;   /* muted sage — nominal, healthy, pass */
  --warning:        #AA8855;   /* amber — time warnings, caution */
  --error:          #CC90A8;   /* rose-bright — failure, danger */
  --dream:          #585878;   /* indigo — altered state, knowledge */
  --dream-dim:      #383858;   /* dimmed indigo */

  /* === GOLD (web-only — replaces bone for interactive elements) === */
  --gold:           #C9A84C;   /* primary brand — CTAs, active states */
  --gold-dim:       rgba(201, 168, 76, 0.20);  /* borders at rest */
  --gold-mid:       rgba(201, 168, 76, 0.40);  /* borders on hover */
  --gold-glow:      rgba(201, 168, 76, 0.05);  /* hover backgrounds */

  /* === CRT MATERIALITY (atmospheric effects) === */
  --phosphor:       #1A1018;   /* ghost of recently-bright pixels */
  --noise-warm:     #2A1820;   /* warm noise in degraded states */
  --noise-cool:     #201828;   /* cool noise in calm states */
  --scanline:       #050507;   /* darkened scanline rows */

  /* === GLASSMORPHISM === */
  --glass-1-bg:     rgba(255, 255, 255, 0.02);
  --glass-1-border: rgba(255, 255, 255, 0.05);
  --glass-1-blur:   12px;
  --glass-2-bg:     rgba(255, 255, 255, 0.04);
  --glass-2-border: rgba(255, 255, 255, 0.08);
  --glass-2-blur:   20px;
  --glass-3-bg:     rgba(255, 255, 255, 0.06);
  --glass-3-border: rgba(255, 255, 255, 0.12);
  --glass-3-blur:   30px;
}
```

### 1.2 Typography (`fonts.css`)

Three fonts, three purposes. Serif for authority. Mono for data and truth. Sans for comfortable reading.

```css
/* Instrument Serif — display headlines, hero text, section titles */
@import url('https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&display=swap');

/* JetBrains Mono — addresses, numbers, code, data values, terminal */
@import url('https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;600;700&display=swap');

/* General Sans — body copy, UI text, navigation */
/* Via FontShare CDN */
@import url('https://api.fontshare.com/v2/css?f[]=general-sans@400,500,600,700&display=swap');

:root {
  --font-serif: 'Instrument Serif', serif;
  --font-sans:  'General Sans', -apple-system, sans-serif;
  --font-mono:  'JetBrains Mono', 'SF Mono', monospace;
}
```

**Type scale** (deliberately restrained):

| Name | Size | Weight | Font | Usage |
|------|------|--------|------|-------|
| display | 48px | 400 | serif | Page hero only (one per page max) |
| h1 | 32px | 500 | serif | Section headers |
| h2 | 24px | 500 | sans | Subsection headers |
| h3 | 18px | 500 | sans | Card headers, group labels |
| body | 16px | 400 | sans | Body copy, descriptions |
| small | 14px | 400 | sans | Captions, metadata |
| mono | 14px | 400 | mono | Data values, code, terminal output, numbers |
| mono-sm | 12px | 400 | mono | Dense data tables, sparklines |

### 1.3 Spacing

8px base grid. Padding follows powers of 2 on the base:

```css
:root {
  --space-1:  4px;
  --space-2:  8px;
  --space-3:  12px;
  --space-4:  16px;
  --space-5:  24px;
  --space-6:  32px;
  --space-7:  48px;
  --space-8:  64px;
}
```

### 1.4 Animation

```css
:root {
  /* Easing */
  --ease-luxury:  cubic-bezier(0.22, 1, 0.36, 1);     /* all standard transitions */
  --ease-subtle:  cubic-bezier(0.25, 0.46, 0.45, 0.94); /* micro-interactions */
  --ease-bounce:  cubic-bezier(0.34, 1.56, 0.64, 1);   /* emphasis, used sparingly */

  /* Durations */
  --dur-flash:    200ms;   /* value-change flash */
  --dur-fade:     400ms;   /* phosphor fade-out */
  --dur-page:     250ms;   /* page transition */
  --dur-stagger:  50ms;    /* per-item stagger delay */
  --dur-glow:     2000ms;  /* phosphor afterimage linger */
  --dur-breath:   4000ms;  /* heartbeat/breathing cycle (modulated by state) */
}

/* Reduced motion: disable everything except opacity */
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }
}
```

### 1.5 Atmospheric Layers (`global.css`)

Four CSS layers replace the TUI's 8-layer frame stack:

```css
/* Layer 1: Noise floor — constant low-opacity texture */
body::before {
  content: '';
  position: fixed;
  inset: 0;
  z-index: 0;
  opacity: 0.015;
  background-image: url("data:image/svg+xml,..."); /* fractal noise SVG */
  pointer-events: none;
  mix-blend-mode: screen;
}

/* Layer 2: Scanline hint — every 3rd pixel row darkened */
body::after {
  content: '';
  position: fixed;
  inset: 0;
  z-index: 0;
  opacity: 0.03;
  background: repeating-linear-gradient(
    0deg,
    transparent 0px,
    transparent 2px,
    var(--scanline) 2px,
    var(--scanline) 3px
  );
  pointer-events: none;
}

/* Layer 3: Content (normal DOM flow) */
/* Layer 4: Glow overlay (per-component box-shadow / filter) */
```

The noise and scanline are **barely perceptible**. They create depth without being distracting. At `opacity: 0.015` and `0.03` respectively, they read as texture, not effect. This is the ROSEDUST principle: the medium acknowledges itself.

---

## 2. Component Architecture

### 2.1 Directory Structure

```
demo/demo-app/
  package.json
  tsconfig.json
  vite.config.ts
  index.html
  src/
    main.tsx                          # StrictMode + createRoot
    App.tsx                           # BrowserRouter + routes

    styles/
      rosedust.css                    # §1.1 color tokens
      fonts.css                       # §1.2 typography
      global.css                      # §1.5 reset + atmospheric layers
      glass.css                       # glassmorphism utility classes
      animations.css                  # keyframes: phosphor flash, breathing, fade-in

    hooks/
      useApi.ts                       # fetch wrapper with base URL resolution
      useServerHealth.ts              # /health polling → connection status
      useTerminal.ts                  # xterm.js lifecycle + WS bridge + resize
      useSSE.ts                       # Server-Sent Events for live data streams
      usePhosphor.ts                  # value-change detection → flash class application
      useBreathing.ts                 # rAF loop driving --breath-phase CSS var (0→2π)

    components/
      Layout/
        Layout.tsx                    # Chrome: top bar + sidebar nav + status bar + <Outlet>
        Layout.css
        TopBar.tsx                    # ⌈ NUNCHI ⌋ wordmark + nav tabs + health indicator
        StatusBar.tsx                 # Bottom: tick counter, connection, breadcrumb

      Glass/
        GlassCard.tsx                 # Three-level glassmorphism card (level 1/2/3)
        GlassPanel.tsx                # Full-bleed glass panel for sections

      Data/
        PhosphorNumber.tsx            # THE core primitive — number with flash-on-change
        PhosphorNumber.css
        StatCard.tsx                  # Label + PhosphorNumber + optional sparkline
        StatCard.css
        ErosionBar.tsx                # Horizontal fill bar with decay animation
        ErosionBar.css
        ConfidenceBadge.tsx           # Pass/fail/partial badge with gate semantics
        DataTable.tsx                 # Scrolling table with phosphor row highlights
        DataTable.css

      Terminal/
        TerminalPane.tsx              # Single xterm.js pane with ROSEDUST theme
        TerminalPane.css
        TerminalGrid.tsx              # N-up grid with ResizeObserver
        TerminalGrid.css

      Charts/
        CostChart.tsx                 # Canvas 2D cumulative cost over time
        CostChart.css
        BarChart.tsx                  # Canvas 2D horizontal comparison bars
        BarChart.css
        Sparkline.tsx                 # Inline SVG sparkline (20-40 data points)
        Sparkline.css
        TerrainMap.tsx                # Canvas 2D contour map for knowledge topology
        TerrainMap.css

      Timeline/
        Timeline.tsx                  # Vertical step timeline with status indicators
        Timeline.css
        CommandLog.tsx                # Scrolling log with phosphor aging
        CommandLog.css

      Atmosphere/
        PulseGlobe.tsx               # three-globe + Three.js hero animation (lazy-loaded)
        NoiseOverlay.tsx              # SVG noise floor (atmospheric layer 1)
        BreathingBorder.tsx           # Border that pulses with --breath-phase

    pages/
      Home.tsx                        # Landing: hero + four primitives + nav cards
      Home.css
      Demo.tsx                        # 7-scenario pitch demo with playback
      Demo.css
      Terminal.tsx                    # Multi-pane PTY terminal
      Terminal.css
      Explorer.tsx                    # Health + episodes + events + knowledge
      Explorer.css
      Bench.tsx                       # Benchmark lab: configure + run + compare
      Bench.css
      BenchLive.tsx                   # Live benchmark monitor with cost chart
      BenchLive.css
      Share.tsx                       # Shareable execution receipt (the URL artifact)
      Share.css

    lib/
      serve-url.ts                    # SERVE_URL / WS_BASE resolution
      rosedust-theme.ts               # xterm.js ITheme using ROSEDUST colors
      demo-scenarios.ts               # 7 demo scenario definitions
      format.ts                       # Number formatting, duration formatting
```

### 2.2 Core Primitive: `PhosphorNumber`

This is the single most important component. Every number in the app renders through it. It implements the ROSEDUST phosphor decay chain: when a value changes, the cell flashes, settles, then leaves a fading afterimage.

```tsx
interface PhosphorNumberProps {
  value: number;
  format?: (n: number) => string;     // e.g., formatUSD, formatPercent
  size?: 'sm' | 'md' | 'lg';         // mono-sm, mono, h3
  bone?: boolean;                      // true = render in --bone (once per view!)
  className?: string;
}
```

**Behavior**:
1. On mount: render value in current color, no flash.
2. On value increase: `--success` background flash (200ms) → `--phosphor` afterglow (2s fade) → transparent. Text briefly brightens to `--bone` for one paint.
3. On value decrease: `--rose-bright` background flash (200ms) → `--phosphor` afterglow (2s fade) → transparent. Text dims to `--rose-dim` for one paint.
4. Stale (no change in 60s): gradually dims from semantic color toward `--text-dim`.
5. All transitions use `--ease-luxury`.

**CSS keyframes**:
```css
@keyframes phosphor-up {
  0%   { background: var(--success); color: var(--bone); }
  30%  { background: var(--phosphor); }
  100% { background: transparent; }
}
@keyframes phosphor-down {
  0%   { background: var(--rose-bright); color: var(--rose-dim); }
  30%  { background: var(--phosphor); }
  100% { background: transparent; }
}
```

### 2.3 Core Primitive: `GlassCard`

Three elevation levels following the glassmorphism system:

```tsx
interface GlassCardProps {
  level?: 1 | 2 | 3;               // glassmorphism intensity
  breathing?: boolean;               // border opacity pulses with --breath-phase
  children: React.ReactNode;
  className?: string;
}
```

Level 1 (subtle): `background: var(--glass-1-bg)`, `border: 1px solid var(--glass-1-border)`, `backdrop-filter: blur(var(--glass-1-blur))`.

Level 2 (standard): default. Most cards.

Level 3 (prominent): modals, hero sections, the single most important card.

### 2.4 Core Primitive: `ErosionBar`

Horizontal fill bar that animates like the TUI's `MortalityGauge`. Fill boundary transitions through opacity steps, not instant jumps:

```tsx
interface ErosionBarProps {
  value: number;                     // 0-1
  color?: 'rose' | 'warning' | 'dream' | 'success';
  label?: string;
  showPercent?: boolean;
}
```

The fill uses a gradient: `linear-gradient(90deg, color/30% 0%, color 100%)` — semi-transparent at left, solid at fill boundary. This simulates the TUI's depth/liquid-level effect.

### 2.5 Atmospheric: `useBreathing` Hook

A single `requestAnimationFrame` loop drives organic animation across the entire app:

```ts
function useBreathing(): void {
  useEffect(() => {
    let frame: number;
    const tick = (t: number) => {
      const phase = (t / 1000) * Math.PI * 0.5;  // ~4s full cycle
      // Incommensurate frequencies for organic quality
      const f1 = Math.sin(phase * 1.0);           // primary breath
      const f2 = Math.sin(phase * 1.41);           // secondary (√2, never repeats)
      const f3 = Math.sin(phase * 1.73);           // tertiary (√3)

      const root = document.documentElement;
      root.style.setProperty('--breath', String(f1));
      root.style.setProperty('--breath-2', String(f2));
      root.style.setProperty('--breath-3', String(f3));

      // Border breathing: 0.7 + 0.3 * sin = range [0.4, 1.0]
      root.style.setProperty('--border-breath', String(0.7 + 0.3 * f1));

      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, []);
}
```

This is called once in `Layout.tsx`. All breathing animations consume the CSS custom properties it sets.

---

## 3. Page Specifications

### 3.1 Home (`/`)

**Role**: Landing page. First thing seen when opening `http://localhost:6677`. Must immediately communicate: this is infrastructure, it's running, it's beautiful.

**Layout**: Full-viewport hero with centered content. No sidebar. Atmospheric noise visible.

**Structure**:
```
┌─────────────────────────────────────────────────────┐
│                                                     │
│              ⌈ N U N C H I ⌋                        │  ← Instrument Serif, 48px, --rose
│                                                     │
│     Infrastructure for agents that build themselves  │  ← General Sans, 18px, --text-primary
│                                                     │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌─────┐ │
│  │ Identity │  │   Cost   │  │Knowledge │  │Dura-│ │  ← Four primitive cards
│  │          │  │Prediction│  │          │  │bility│ │     GlassCard level 1
│  │  ◆ On    │  │  ◆ On    │  │  ◆ On    │  │◆ On │ │     Status from /api/health
│  └──────────┘  └──────────┘  └──────────┘  └─────┘ │
│                                                     │
│     roko serve · 115 routes · port 6677 · healthy   │  ← mono-sm, --text-dim
│                                                     │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌───────┐ │
│  │  Demo   │  │Terminal │  │Explorer │  │ Bench │ │  ← Navigation cards
│  │         │  │         │  │         │  │       │ │     GlassCard level 2
│  └─────────┘  └─────────┘  └─────────┘  └───────┘ │     hover: gold-glow bg, gold-mid border
│                                                     │
└─────────────────────────────────────────────────────┘
```

**Key details**:
- `⌈ N U N C H I ⌋` uses fullwidth Unicode characters — the ROSEDUST institutional header pattern
- Four primitive cards pulse gently with `--border-breath` when healthy
- Navigation cards use `--ease-luxury` hover transition: border `--glass-2-border` → `--gold-mid`, background gains `--gold-glow`
- Status line polls `/health` every 5s via `useServerHealth`
- No chart, no terminal, no complexity. Pure restraint.

### 3.2 Demo (`/demo`)

**Role**: The 7-scenario pitch demo. This is the investor-facing page. Each scenario runs a `roko` command in a terminal pane and shows structured output alongside.

**Layout**: Split — terminal left (55%), structured output right (45%).

**Structure**:
```
┌─ TopBar ──────────────────────────────────────────────┐
│ ⌈NUNCHI⌋  Home  [Demo]  Terminal  Explorer  Bench     │
├───────────────────────────────┬────────────────────────┤
│                               │  Scenario: Identity    │  ← h2, scenario selector
│  Terminal Pane                │                        │
│  (xterm.js, ROSEDUST theme)  │  ┌─ Timeline ────────┐ │
│                               │  │ ◆ Identity    ✔   │ │  ← Timeline component
│  $ nunchi run "Fix the bug"  │  │ ◇ Predict     ◆   │ │     shows beat progression
│  ◆ Identity                  │  │ ◇ Gates       ○   │ │
│  │ agent: roko-alpha-7a2     │  │ ◇ Knowledge   ○   │ │
│  │ verified: ✔ (chain)       │  │ ◇ Result      ○   │ │
│  ...                         │  └────────────────────┘ │
│                               │                        │
│                               │  ┌─ StatCards ───────┐ │
│                               │  │ Cost: $0.08       │ │  ← PhosphorNumber, updates live
│                               │  │ Predicted: $0.12  │ │
│                               │  │ Delta: -33%       │ │  ← bone if significant
│                               │  │ Gates: 3/3 ✔      │ │
│                               │  └────────────────────┘ │
│                               │                        │
│                               │  ┌─ CostChart ──────┐ │
│                               │  │ (Canvas 2D)       │ │  ← cumulative cost curve
│                               │  └────────────────────┘ │
├───────────────────────────────┴────────────────────────┤
│  Scenario: [1] [2] [3] [4] [5] [6] [7]   ▶ Auto-play │  ← scenario selector bar
└────────────────────────────────────────────────────────┘
```

**Scenario definitions** (from demo-scenarios.ts, updated):

| # | Name | Command | What It Shows |
|---|------|---------|---------------|
| 1 | Identity | `nunchi run "Fix auth bug"` | Verified agent identity, policy gates |
| 2 | Predict | `nunchi run "Add rate limiter"` | Cost prediction → actual, delta |
| 3 | Knowledge | `nunchi run "Optimize queries"` | Loads knowledge from prior runs |
| 4 | Resume | `nunchi run "Refactor service" && Ctrl-C && nunchi run --resume` | Kill + resume, zero work lost |
| 5 | Fleet | `nunchi agents list` | Multiple agents, different roles |
| 6 | Audit | `nunchi audit --last 5` | Execution audit trail |
| 7 | Share | `nunchi run --share "Deploy fix"` | Produces shareable URL |

**Auto-play mode**: Advances through scenarios on a timer (configurable, default 30s per scenario). Each scenario sends the command to the terminal pane via the WebSocket session. The structured output panel updates as SSE events arrive from the server.

**Key details**:
- Terminal pane uses `useTerminal` hook with ROSEDUST xterm theme
- Structured output updates via SSE from `/api/events` or polling `/api/status`
- Cost chart renders cumulative cost via Canvas 2D — ROSEDUST palette, no axes labels (Tufte minimal)
- Scenario bar uses `⌈1⌋ ⌈2⌋ ⌈3⌋` notation, active scenario in `--rose`, completed in `--success`

### 3.3 Terminal (`/terminal`)

**Role**: Multi-pane PTY terminal. Power user tool. "Here, try it yourself" moment in the a16z meeting.

**Layout**: Configurable grid of terminal panes with controls.

**Structure**:
```
┌─ TopBar ──────────────────────────────────────────────┐
│ ⌈NUNCHI⌋  Home  Demo  [Terminal]  Explorer  Bench     │
├────────────────────────────────────────────────────────┤
│  Layout: [1×1] [1×2] [2×2] [Custom]   + New Pane     │
├───────────────────────┬────────────────────────────────┤
│                       │                                │
│  Terminal Pane 1      │  Terminal Pane 2               │
│  (xterm.js)           │  (xterm.js)                    │
│                       │                                │
│  $ _                  │  $ nunchi status               │
│                       │                                │
│                       │                                │
│                       │                                │
├───────────────────────┴────────────────────────────────┤
│  ● Session 1: idle   ● Session 2: running  [Kill All] │
└────────────────────────────────────────────────────────┘
```

**Key details**:
- Each pane is a `TerminalPane` with its own WebSocket session
- `TerminalGrid` uses CSS Grid with `ResizeObserver` to pass dimensions to xterm.js `addon-fit`
- Layout controls above the grid switch between presets
- Session status bar at bottom shows per-pane state
- Pane borders use `--border` at rest, `--border-active` when focused (click to focus)

### 3.4 Explorer (`/explorer`)

**Role**: Health dashboard, episode browser, event log, knowledge store viewer. The "technical diligence" page.

**Layout**: Four tabs within the page.

**Tabs**:

**Tab 1: Health** — System status overview
```
  ┌─ Server ────────┐  ┌─ Agents ──────────┐  ┌─ Knowledge ───────┐
  │ Status: Healthy  │  │ Active: 3         │  │ Entries: 847      │
  │ Uptime: 4h 23m   │  │ Idle: 1           │  │ Confidence: 0.72  │
  │ Routes: 115      │  │ Total runs: 42    │  │ Sources: 12       │
  └──────────────────┘  └───────────────────┘  └───────────────────┘

  ┌─ Recent Episodes ────────────────────────────────────────────────┐
  │  #42  Fix auth bug        ✔ 3/3 gates   $0.08   2m ago         │  ← phosphor-aged rows
  │  #41  Add rate limiter    ✔ 3/3 gates   $0.11   5m ago         │
  │  #40  Optimize queries    ✖ 2/3 gates   $0.15   12m ago        │
  └──────────────────────────────────────────────────────────────────┘
```

**Tab 2: Episodes** — Detailed episode browser with expandable rows
**Tab 3: Events** — Live event stream (SSE-powered, phosphor-aging log)
**Tab 4: Knowledge** — NeuroStore browser with terrain map visualization

**Knowledge terrain map** (Tab 4): Canvas 2D contour map where elevation = confidence, color = knowledge domain. Uses `d3-contour` for isoline computation, Canvas 2D for rendering. ROSEDUST palette: low confidence = `--void`, medium = `--rose-dim`, high = `--rose`, peak = `--bone-dim`. This is the knowledge visualization described in DEMO-VISUAL-SPEC.md — the visual differentiator.

### 3.5 Bench (`/bench`)

**Role**: Benchmark lab. Configure, run, and compare benchmark results.

**Layout**: Three-panel — config left, results center, history right.

**Structure**:
```
┌─ Config ─────────┐  ┌─ Results ─────────────────┐  ┌─ History ─────┐
│ Benchmark:       │  │                           │  │ Run #1  0.82  │
│ ○ τ-bench        │  │  Score: 0.84              │  │ Run #2  0.79  │
│ ● AppWorld       │  │  Cost:  $0.08             │  │ Run #3  0.84  │
│ ○ GAIA           │  │  Time:  47s               │  │               │
│                  │  │                           │  │               │
│ Tasks: [5]       │  │  ┌─ Cost Chart ─────────┐ │  │               │
│ Budget: [$1.00]  │  │  │ (Canvas 2D)          │ │  │               │
│                  │  │  └──────────────────────┘ │  │               │
│ [▶ Run]          │  │                           │  │               │
└──────────────────┘  └───────────────────────────┘  └───────────────┘
```

### 3.6 BenchLive (`/bench-live`)

**Role**: Live monitoring of a running benchmark. Bloomberg Two-Tape inspired — Roko vs competitor side-by-side.

**Layout**: Full-width with two-column comparison.

**Structure**:
```
┌─ ⌈ LIVE BENCHMARK ⌋ ──────────────────────────────────┐
│                                                        │
│  ┌─ Roko ─────────────────┐  ┌─ LangGraph ──────────┐ │
│  │ Tasks: 3/5  ✔✔✔○○      │  │ Tasks: 1/5  ✔○○○○    │ │
│  │ Cost:  $0.08           │  │ Cost:  $2.40          │ │  ← PhosphorNumber, bone for winner
│  │ Time:  47s             │  │ Time:  2m 12s         │ │
│  │ ────────────────────── │  │ ────────────────────  │ │
│  │ [CostChart]            │  │ [CostChart]           │ │
│  └────────────────────────┘  └───────────────────────┘ │
│                                                        │
│  Winner: Roko (p < 0.01)   Cost ratio: 30:1           │  ← bone for the declaration
│                                                        │
│  ┌─ Task Log ──────────────────────────────────────┐   │
│  │ (phosphor-aging command log)                     │   │
│  └──────────────────────────────────────────────────┘   │
└────────────────────────────────────────────────────────┘
```

### 3.7 Share (`/share/:id`)

**Role**: The artifact that leaves the room. When `nunchi run --share` completes, it produces a URL. This page renders the execution receipt — a computation receipt with full timeline, cost breakdown, and verification.

**Layout**: Centered, single-column, print-friendly. The only page that breaks the dark theme — the receipt card itself uses a cream/bone background for contrast and printability.

**Structure**:
```
┌─────────────────────────────────────────────────┐
│  (dark void background)                         │
│                                                 │
│  ┌─ Receipt Card (cream bg) ──────────────────┐ │
│  │                                            │ │
│  │  ⌈ COMPUTATION RECEIPT ⌋                   │ │
│  │                                            │ │
│  │  Agent: roko-alpha-7a2                     │ │
│  │  Task: "Fix the authentication bug"        │ │
│  │  Started: 2026-04-28T14:23:07Z             │ │
│  │  Duration: 2m 14s                          │ │
│  │                                            │ │
│  │  ── Cost Breakdown ──────────────────────  │ │
│  │  Input tokens:    12,847     $0.038        │ │
│  │  Output tokens:    4,231     $0.042        │ │
│  │  Total:                      $0.080        │ │
│  │  Predicted:                  $0.120        │ │
│  │  Delta:                     -33.3%         │ │
│  │                                            │ │
│  │  ── Gates ───────────────────────────────  │ │
│  │  Compile:  ✔ pass                          │ │
│  │  Test:     ✔ pass (47/47)                  │ │
│  │  Clippy:   ✔ pass (0 warnings)             │ │
│  │                                            │ │
│  │  ── Timeline ────────────────────────────  │ │
│  │  [vertical step timeline]                  │ │
│  │                                            │ │
│  │  ── Verification ────────────────────────  │ │
│  │  Hash: 0x7a2f...3c4d                       │ │
│  │  Chain: Nunchi L1 (block #12847)           │ │
│  │                                            │ │
│  │  [Download PDF]  [View on Explorer]        │ │
│  └────────────────────────────────────────────┘ │
│                                                 │
└─────────────────────────────────────────────────┘
```

**Key details**:
- The receipt card itself uses `--bone` as background color with dark text — the hybrid dark+cream pattern from DEMO-VISUAL-SPEC.md
- The surrounding page stays dark (void background with noise)
- The contrast between void and cream receipt card is the visual impact — this is "the thing that leaves the room"
- PDF download renders the same layout via `html2canvas` or server-side PDF generation
- The receipt is the only place where bone is used as a background, not a text color

---

## 4. Rust Integration

### 4.1 `crates/roko-serve/src/embedded.rs`

Unchanged from existing — `rust-embed` bakes `demo/demo-app/dist/` into the binary. SPA fallback serves `index.html` for all non-API routes.

```rust
#[derive(rust_embed::Embed)]
#[folder = "../../demo/demo-app/dist/"]
struct DemoAssets;

pub async fn serve_embedded(req: Request<Body>) -> Response<Body> {
    let path = req.uri().path().trim_start_matches('/');
    let file = DemoAssets::get(path)
        .or_else(|| DemoAssets::get("index.html"));
    match file {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .header("content-type", mime.as_ref())
                .header("cache-control", "public, max-age=3600")
                .body(Body::from(content.data.into_owned()))
                .unwrap()
        }
        None => Response::builder()
            .status(404)
            .body(Body::from("not found"))
            .unwrap(),
    }
}
```

### 4.2 Route Integration

API routes (`/api/*`, `/ws/*`, `/health`) take precedence via Axum's router. The embedded handler is registered as a fallback:

```rust
let app = router
    .merge(api_routes)      // /api/*, /ws/*, /health
    .fallback(serve_embedded);
```

### 4.3 `build.rs`

Triggers `npm run build` during `cargo build`. Gracefully skips if Node.js is not installed or `SKIP_FRONTEND_BUILD` is set.

### 4.4 Vite Config

```ts
export default defineConfig({
  plugins: [react()],
  base: '/',
  build: {
    outDir: 'dist',
    sourcemap: false,
    // Target modern browsers only — no IE11
    target: 'es2022',
  },
  server: {
    proxy: {
      '/api': 'http://localhost:6677',
      '/ws': { target: 'ws://localhost:6677', ws: true },
      '/health': 'http://localhost:6677',
    },
  },
});
```

---

## 5. Implementation Phases

### Phase 0: Scaffold + Design System (1-2 days)

**Goal**: Empty SPA shell with full design system, serving from `roko serve`.

**Files**:
- `package.json` — React 19, react-router 7, xterm 5.5, Vite 6, TypeScript 5
- `vite.config.ts` — proxy config
- `index.html` — SPA shell with `<div id="root">`
- `src/main.tsx` — StrictMode + createRoot
- `src/App.tsx` — BrowserRouter with placeholder routes
- `src/styles/rosedust.css` — full §1.1 token set
- `src/styles/fonts.css` — full §1.2 font imports
- `src/styles/global.css` — reset + atmospheric layers (§1.5)
- `src/styles/glass.css` — `.glass-1`, `.glass-2`, `.glass-3` utility classes
- `src/styles/animations.css` — phosphor keyframes, fade-in, breathing
- `src/components/Layout/Layout.tsx` — chrome with TopBar + StatusBar + Outlet
- `src/components/Layout/TopBar.tsx` — `⌈ NUNCHI ⌋` + nav tabs
- `src/components/Layout/StatusBar.tsx` — connection status
- `src/components/Glass/GlassCard.tsx` — three-level card
- `src/hooks/useServerHealth.ts` — /health polling
- `src/hooks/useBreathing.ts` — rAF breathing loop
- `src/lib/serve-url.ts` — URL resolution

**Verify**: `npm run dev` shows the shell with atmospheric noise, scanlines, glass cards, breathing borders. `cargo build -p roko-serve` embeds the dist. `roko serve` serves the SPA.

### Phase 1: Core Primitives + Home (1-2 days)

**Goal**: PhosphorNumber, StatCard, ErosionBar, and the Home landing page.

**Files**:
- `src/components/Data/PhosphorNumber.tsx` + CSS
- `src/components/Data/StatCard.tsx` + CSS
- `src/components/Data/ErosionBar.tsx` + CSS
- `src/components/Data/ConfidenceBadge.tsx`
- `src/hooks/usePhosphor.ts`
- `src/hooks/useApi.ts`
- `src/pages/Home.tsx` + CSS
- `src/lib/format.ts`

**Verify**: Home page renders with four primitive cards showing live health status. PhosphorNumber flashes on value change. ErosionBar fills correctly. Atmospheric layers visible.

### Phase 2: Terminal + Demo (2-3 days)

**Goal**: Terminal panes working over WebSocket. Demo page with scenario progression.

**Files**:
- `src/hooks/useTerminal.ts` — xterm lifecycle + WS bridge
- `src/lib/rosedust-theme.ts` — xterm ITheme
- `src/components/Terminal/TerminalPane.tsx` + CSS
- `src/components/Terminal/TerminalGrid.tsx` + CSS
- `src/components/Timeline/Timeline.tsx` + CSS
- `src/components/Timeline/CommandLog.tsx` + CSS
- `src/pages/Terminal.tsx` + CSS
- `src/pages/Demo.tsx` + CSS
- `src/lib/demo-scenarios.ts`

**Verify**: Terminal page opens PTY sessions via WebSocket, resizes correctly. Demo page runs scenarios, terminal shows command output, structured panel shows timeline + stats updating live.

### Phase 3: Explorer + Data Visualization (2-3 days)

**Goal**: Health dashboard, episode browser, event log, and knowledge terrain map.

**Files**:
- `src/hooks/useSSE.ts` — Server-Sent Events
- `src/components/Data/DataTable.tsx` + CSS
- `src/components/Charts/Sparkline.tsx` + CSS
- `src/components/Charts/TerrainMap.tsx` + CSS
- `src/pages/Explorer.tsx` + CSS

**Verify**: Explorer shows live health data. Episodes load and display with phosphor aging. Knowledge tab shows terrain map. SSE events stream in real-time.

### Phase 4: Bench + BenchLive (2-3 days)

**Goal**: Benchmark lab and live monitoring.

**Files**:
- `src/components/Charts/CostChart.tsx` + CSS
- `src/components/Charts/BarChart.tsx` + CSS
- `src/pages/Bench.tsx` + CSS
- `src/pages/BenchLive.tsx` + CSS

**Verify**: Bench page configures and launches benchmarks. BenchLive shows side-by-side comparison with live cost charts updating via SSE.

### Phase 5: Share + Polish (2-3 days)

**Goal**: Shareable computation receipt page. Pulse Globe hero (lazy-loaded). Final polish.

**Files**:
- `src/pages/Share.tsx` + CSS
- `src/components/Atmosphere/PulseGlobe.tsx` (lazy import, Three.js)
- Polish: transitions between pages (250ms fade), stagger animations on card lists, responsive breakpoints

**Verify**: Share page renders receipt with cream card on dark background. PDF download works. Pulse Globe loads and renders on Home page. All pages transition smoothly.

### Phase 6: Cleanup (0.5 day)

- Remove `demo/demo-web/` HTML files or archive them
- Update `.gitignore` for `node_modules/`, `dist/`
- Remove old `root_index()` handler from routes if still present
- Verify `cargo build --release` produces working binary with embedded SPA

---

## 6. Dependencies

### npm

```json
{
  "dependencies": {
    "react": "^19.1.0",
    "react-dom": "^19.1.0",
    "react-router": "^7.6.0"
  },
  "devDependencies": {
    "@types/react": "^19.1.0",
    "@types/react-dom": "^19.1.0",
    "@vitejs/plugin-react": "^4.4.0",
    "@xterm/xterm": "^5.5.0",
    "@xterm/addon-fit": "^0.10.0",
    "typescript": "^5.8.0",
    "vite": "^6.3.0"
  }
}
```

**Optional (Phase 5 only)**:
- `three` + `three-globe` — for Pulse Globe hero. Lazy-loaded, not on critical path.
- `d3-contour` — for terrain map isolines. Small, tree-shakeable.

**Explicitly NOT included**:
- No CSS framework (Tailwind, etc.) — custom properties + vanilla CSS
- No charting library (recharts, chart.js) — Canvas 2D direct
- No UI component library (Radix, shadcn) — bespoke glassmorphic components
- No state management library — React context + hooks
- No SSR framework — pure SPA, Axum is the backend

### Cargo

```toml
# crates/roko-serve/Cargo.toml
rust-embed = { version = "8", features = ["debug-embed"] }
mime_guess = "2"
```

`debug-embed` reads from disk in dev builds — change files, rebuild Vite, refresh browser. No Rust recompile needed for frontend changes during development.

---

## 7. ROSEDUST Color Rules (Non-Negotiable)

These rules are taken directly from the 18-interfaces design system and adapted for web:

1. **Rose dominates**. 80% of colored elements on any screen use the rose family. One-color-dominant.
2. **Bone appears at most once per view**. If nothing is critical, bone does not appear. Bone is earned, not decorative. On web, `--gold` may substitute for interactive elements (buttons, links).
3. **Background is `#060608` or `#0A0A0A`, never `#000000`**. Pure black is a hole. The void has a violet undertone.
4. **Brightest element is `--rose-bright` (`#CC90A8`)**. Never white. Never `#FFFFFF`. The brightest text is `--text-primary` (`#988090`).
5. **Color transitions are always gradual**. Nothing snaps. Use `--ease-luxury` for all transitions. Minimum `200ms` duration.
6. **50% void rule**: Content areas have generous padding. Dense data is the exception (tables), not the rule. Whitespace gives data meaning.
7. **The phosphor chain is law**: Value changes flash → settle → afterglow → fade. Not instant updates. Every number has memory.

---

## 8. File Inventory

Total new files: ~48 (matching existing count — complete rewrite, not additive).

| Category | Files | Purpose |
|----------|-------|---------|
| Styles | 5 | rosedust.css, fonts.css, global.css, glass.css, animations.css |
| Layout | 4 | Layout.tsx/css, TopBar.tsx, StatusBar.tsx |
| Glass | 2 | GlassCard.tsx, GlassPanel.tsx |
| Data | 8 | PhosphorNumber, StatCard, ErosionBar, ConfidenceBadge, DataTable (each with CSS) |
| Terminal | 4 | TerminalPane, TerminalGrid (each with CSS) |
| Charts | 8 | CostChart, BarChart, Sparkline, TerrainMap (each with CSS) |
| Timeline | 4 | Timeline, CommandLog (each with CSS) |
| Atmosphere | 3 | PulseGlobe, NoiseOverlay, BreathingBorder |
| Pages | 14 | Home, Demo, Terminal, Explorer, Bench, BenchLive, Share (each with CSS) |
| Hooks | 6 | useApi, useServerHealth, useTerminal, useSSE, usePhosphor, useBreathing |
| Lib | 4 | serve-url, rosedust-theme, demo-scenarios, format |
| Config | 4 | main.tsx, App.tsx, vite.config.ts, tsconfig.json |

---

## 9. Verification Checklist

Each phase must pass these checks before moving to the next:

- [ ] `npm run dev` serves the app with HMR, proxied API calls work
- [ ] `npm run build` produces a clean `dist/` directory
- [ ] `cargo build -p roko-serve` triggers `npm run build` and embeds the dist
- [ ] `roko serve` serves the SPA from embedded assets on `:6677`
- [ ] Client-side routing works (navigating to `/demo` directly serves the SPA, not 404)
- [ ] API routes (`/api/*`, `/ws/*`, `/health`) still function through the SPA
- [ ] WebSocket terminal sessions connect and resize correctly
- [ ] Atmospheric layers (noise + scanlines) are visible but subtle
- [ ] PhosphorNumber flashes on value change with correct colors
- [ ] Glass cards show backdrop-filter blur at three levels
- [ ] `prefers-reduced-motion` disables all animations except opacity
- [ ] All text meets WCAG AA contrast ratios against `--surface-0` background
- [ ] No console errors or React warnings in production build

---

## 10. What This Plan Does NOT Cover

These are future work, documented in DEMO-BUILD.md tiers 2-3:

- **Pulse Globe** (Three.js): Phase 5 optional. The globe is impressive but heavy — only worth including if the demo needs a cold-open hero.
- **Terrain knowledge map** (d3-contour): Phase 3. Implementation depends on the NeuroStore API returning topology-friendly data. May need backend work.
- **Bloomberg Two-Tape live benchmark widget**: Requires the HAL harness to be operational. The UI is ready (BenchLive page) but the backend benchmark runner is a separate workstream.
- **Chain view** (block explorer): Phase 2+ of the chain integration. Requires `mirage-rs` or real chain.
- **PDF generation** for computation receipt: Can use `html2canvas` client-side or server-side PDF rendering. Deferred to Phase 5.
- **Custom "Nunchi Mono" typeface**: High effort, high signal. Worth doing eventually but not on the demo critical path.
- **Berkeley Mono license**: $75, trivial. Purchase and configure in terminal emulator (Ghostty) before the pitch meeting. Not a code change.

---

*This document is the implementation companion to DEMO-VISUAL-SPEC.md (what it looks like), DEMO-BUILD.md (what's missing), and DEMO-FLOW.md (what happens during the demo). Together they form the complete build specification for the Nunchi Series A demo.*
