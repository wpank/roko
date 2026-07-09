# 04. Design System — ROSEDUST v2

Design tokens, component specs, animation system, and atmospheric layers. Synthesized from the next-gen spec (doc 11), game UX design system (doc 13), and bardo TUI PRDs.

---

## 1. Design Tokens

```css
:root {
  /* ─── Backgrounds ─── */
  --bg-void: #060608;
  --bg-raised: #0a0810;
  --bg-mid: #080810;
  --bg-deeper: #040406;
  --bg-glass: rgba(8, 8, 12, 0.45);
  --bg-glass-hover: rgba(58, 32, 48, 0.14);
  --bg-glass-active: rgba(58, 32, 48, 0.32);

  /* ─── Borders ─── */
  --border: rgba(255, 255, 255, 0.07);
  --border-soft: rgba(255, 255, 255, 0.04);
  --border-strong: rgba(255, 255, 255, 0.14);
  --border-active: var(--rose-glow);

  /* ─── Rose spectrum ─── */
  --rose: #aa7088;
  --rose-bright: #cc90a8;
  --rose-glow: #dca5bd;
  --rose-dim: #7a5060;
  --rose-deep: #3a2030;
  --rose-ember: #482838;

  /* ─── Bone spectrum ─── */
  --bone: #c8b890;
  --bone-bright: #d8c8a0;
  --bone-dim: #8a7a5a;

  /* ─── Text (BRIGHTENED for web — higher contrast than TUI) ─── */
  --text-primary: #e8dce8;    /* body text, ~7:1 vs void */
  --text-strong: #f8f0f8;     /* headings, ~12:1 vs void */
  --text-soft: #c8b8c4;       /* secondary content */
  --text-dim: #9a8a98;        /* labels, still readable at 12px */
  --text-ghost: #605060;      /* placeholders, hints */

  /* ─── Semantic ─── */
  --dream: #7a7a98;
  --dream-bright: #9494b4;
  --dream-deep: #282848;
  --success: #7a8a78;
  --warning: #c89a68;
  --danger: #cc5555;

  /* ─── Status colors ─── */
  --status-idle: #6a5a68;
  --status-active: #2dd4bf;
  --status-success: #4ade80;
  --status-warning: #fbbf24;
  --status-error: #fb7185;
  --status-blocked: #a78bfa;

  /* ─── Status glows ─── */
  --glow-active: 0 0 12px rgba(45, 212, 191, 0.3);
  --glow-success: 0 0 12px rgba(74, 222, 128, 0.3);
  --glow-error: 0 0 12px rgba(251, 113, 133, 0.3);
  --glow-ambient: 0 0 20px rgba(255, 255, 255, 0.05);
  --glow-rose: 0 0 20px rgba(170, 112, 136, 0.15);

  /* ─── Typography ─── */
  --mono: "JetBrains Mono", ui-monospace, monospace;
  --display: "Fraunces", "Times New Roman", serif;
  --sans: "General Sans", "Inter", system-ui, sans-serif;

  /* ─── Spacing ─── */
  --gap-xs: 4px;
  --gap-sm: 8px;
  --gap-md: 16px;
  --gap-lg: 24px;
  --gap-xl: 40px;
  --gap-2xl: 64px;

  /* ─── Shadows ─── */
  --shadow-sm: 0 1px 3px rgba(0,0,0,0.4), inset 0 1px 0 rgba(255,255,255,0.06);
  --shadow-md: 0 4px 16px rgba(0,0,0,0.5), inset 0 1px 0 rgba(255,255,255,0.06);
  --shadow-lg: 0 12px 40px rgba(0,0,0,0.6);
  --shadow-glow-rose: 0 0 0 1px rgba(220,165,189,0.3), 0 0 20px rgba(170,112,136,0.15);

  /* ─── Motion ─── */
  --ease-snappy: cubic-bezier(0.2, 0.8, 0.2, 1);
  --ease-expo: cubic-bezier(0.16, 1, 0.3, 1);
  --ease-out: cubic-bezier(0, 0, 0.2, 1);
  --duration-instant: 80ms;
  --duration-fast: 150ms;
  --duration-normal: 220ms;
  --duration-slow: 350ms;

  /* ─── Focus ─── */
  --focus-ring: 0 0 0 2px var(--bg-void), 0 0 0 4px rgba(220,165,189,0.7);

  /* ─── Cell ─── */
  --cell-radius: 6px;
  --cell-padding: 10px 12px;
  --cell-gap: 8px;
  --cell-border: 1px solid var(--border);
}
```

---

## 2. Typography Scale

```
Display:    Fraunces italic 300, 48-82px, tracking -0.022em, lh 1.1
Section:    Fraunces italic 400, 32px, tracking -0.012em, lh 1.15
Hero:       Fraunces italic 300, 24px, tracking -0.008em, lh 1.4
Body:       General Sans 400, 16px, lh 1.6
Body large: General Sans 400, 18px, lh 1.6

Label:      JetBrains Mono 500, 12px, tracking 0.08em, uppercase
Label sm:   JetBrains Mono 500, 10px, tracking 0.06em, uppercase
Mono value: JetBrains Mono 400, 14px, tracking 0.02em
Metric lg:  Fraunces italic 400, 38px, tracking -0.015em
Metric sm:  Fraunces italic 400, 32px, tracking -0.012em
```

Minimum sizes: body 14px, table 12px, labels 12px, canvas 10px.
Font weights: body 400, labels 500, headings 500-600, metrics 700.

---

## 3. Status Visual Language

| Status | Color | Dot | Ring | Badge |
|--------|-------|-----|------|-------|
| idle/pending | `--status-idle` | ○ | gray border | `pending` |
| active/running | `--status-active` | ● pulsing | teal + glow | `working` |
| done/success | `--status-success` | ● | green border | `done` |
| failed/error | `--status-error` | ● | rose + glow | `failed` |
| blocked | `--status-blocked` | ● | purple border | `blocked` |

**Color is always redundant.** Never use color alone — pair with icon (✓ ✕ ◉ ○) and text label.

---

## 4. Animation System

### 4.1 Core Keyframes

```css
/* LED pulse — status dots, active indicators */
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}
/* 2.4s ease-in-out infinite */

/* Element entrance — panes, cards, list items */
@keyframes fadeUp {
  from { opacity: 0; transform: translateY(12px); }
}
/* 200ms var(--ease-expo) forwards */
/* Stagger: calc(var(--i, 0) * 40ms) */

/* Loading shimmer */
@keyframes shimmer {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}
/* 1.8s ease-in-out infinite */

/* Value change highlight */
@keyframes value-flash {
  0% { color: var(--bone-bright); text-shadow: 0 0 8px rgba(216,200,160,0.3); }
  100% { color: inherit; text-shadow: none; }
}
/* 300ms var(--ease-out) */

/* Gate pass celebration */
@keyframes gate-pass {
  0% { transform: scale(0.8); opacity: 0; }
  60% { transform: scale(1.05); }
  100% { transform: scale(1); opacity: 1; }
}
/* 200ms var(--ease-snappy) */

/* Phase rail line draw */
@keyframes line-draw {
  from { transform: scaleX(0); transform-origin: left; }
  to { transform: scaleX(1); }
}
/* 300ms var(--ease-out) */

/* Error border flash */
@keyframes error-flash {
  0% { border-color: var(--status-error); box-shadow: var(--glow-error); }
  100% { border-color: var(--border); box-shadow: none; }
}
/* 400ms ease-out */
```

### 4.2 Animation Rules (Hard Constraints)

- Never exceed 400ms for UI feedback
- Use `--ease-out` or `--ease-snappy` for hover/interaction — never `ease` (slow start = perceived latency)
- `will-change: transform, opacity` on hover-animating elements only
- Always `transition: specific-prop`, never `transition: all`
- Respect `prefers-reduced-motion`: disable all except color transitions
- Performance budget: ≤8 concurrent CSS animations, ≤12 Motion springs, ≤4ms/frame for animation

### 4.3 Craft Details

1. **Specular top-edge highlight** on every elevated surface: `inset 0 1px 0 rgba(255,255,255,0.06)`
2. **Borders use rgba white** (`rgba(255,255,255,0.07)`) — adapts to any background
3. **Hover transforms are small**: `translateY(-2px)` + `scale(1.005)`, not theatrical
4. **Active/pressed asymmetric timing**: press 50ms (snappy), release 120ms (smooth)
5. **Value change flash**: 200ms highlight draws eye on metric updates (defeat change blindness)
6. **Staggered list entrance**: 40ms delay between items (fast enough to not feel slow)
7. **Glass panels use `saturate(180%)`** in backdrop-filter for vivid blur
8. **Tooltip entrance**: opacity + 4px Y travel + 0.97 scale (physical feel)

---

## 5. Atmospheric Layers

### 5.1 Grain Texture (most important)

```css
.grain-overlay {
  position: fixed; inset: 0;
  pointer-events: none; z-index: 9999;
  opacity: 0.04; mix-blend-mode: overlay;
  filter: url(#noise);
}
```
SVG filter: `feTurbulence baseFrequency="0.65" numOctaves="3" stitchTiles="stitch"` + `feColorMatrix saturate="0"`.

### 5.2 Scanlines

```css
.scanlines {
  position: fixed; inset: 0;
  pointer-events: none; z-index: 9998;
  background: repeating-linear-gradient(0deg, transparent 0px, transparent 2px, rgba(0,0,0,0.45) 2px, rgba(0,0,0,0.45) 3px);
  opacity: 0.06;
}
```

### 5.3 Vignette

```css
.vignette {
  position: fixed; inset: 0;
  pointer-events: none; z-index: 9997;
  background: radial-gradient(ellipse at center, transparent 50%, rgba(6,6,8,0.72) 100%);
}
```

All layers: `pointer-events: none`, fixed position, negligible GPU cost.

---

## 6. Component Specs

### Pane — Glass Panel

```
┌─ ● LABEL ──────────────── badge ─┐
│                                   │
│  content                          │
│                                   │
├───────────────────────────────────┤
│  footer                           │
└───────────────────────────────────┘
```
- LED dot + mono 10.5px uppercase label
- Glass: `backdrop-filter: blur(12px) saturate(180%)`
- Left rose border: `2px solid var(--rose-dim)` with glow
- Specular: `inset 0 1px 0 rgba(255,255,255,0.06)`
- Hover: border → `--border-strong`, 150ms

### Mosaic — 1px-Gap Metric Grid

- Gap: 1px with `--border` showing through
- Cell padding: 16px 14px (dense — content-first, not decorative)
- Label: mono 10px uppercase, dim
- Value: Fraunces italic 400, 38px, bone-bright
- Sub: Fraunces 300, 14px, soft
- Entrance: 40ms stagger per cell

### GateBar — Gate Status Strip

```
✓ COMPILE   ✓ TEST   ◉ CLIPPY   ○ DIFF
```
- Pass: success + 150ms scale-up (0.8→1.0)
- Fail: rose-glow + 200ms flash
- Running: bone + LED pulse
- Pending: `--text-ghost`
- Mono 10px uppercase, gap 24px

### StatusBadge — Universal Status Indicator

```tsx
<StatusBadge status="active" />
```
Same visual in task rows, agent cards, plan cards, bench runs, gate results. One component, consistent everywhere.

### PhaseRail — Horizontal Step Indicator

```
● IDEA ——— ● PRD ——— ● PLAN ——— ◉ TASKS ——— ○ RUN ——— ○ DONE
```
- Done: filled success, solid line
- Current: rose-glow ring + pulse, rose line
- Pending: dim outline, soft line
- Transition: new dot fills with 200ms scale-up, line "draws" 300ms ease-out

### EmptyState — Informative Empty

- Centered, mono 12px, dim
- Message: what's empty ("No benchmark runs yet")
- Action: what to do (`Run roko bench start to create one`)
- Hint: technical detail (`API returned 404`)

### StatusPill — Connection Indicator (TopNav)

- `live`: green LED + "LIVE 8h 16m"
- `seed`: bone LED + "SEED DATA"
- `reconnecting`: amber pulse + "RECONNECTING..."
- `offline`: dim LED + "OFFLINE"

---

## 7. From Bardo: Transplantable Concepts

The bardo TUI PRDs defined rich visual concepts. These translate to CSS/React as:

| Bardo Concept | Web Translation |
|---------------|----------------|
| 32 interpolating variables | CSS custom properties + Motion springs |
| Three timescales (fast/medium/slow) | Concurrent CSS keyframes at 300ms / 5s / 300s |
| PAD modulation (pleasure/arousal/dominance) | Hue/saturation/brightness CSS filters driven by system state |
| Lifecycle degradation | Conditional CSS classes reducing visual complexity |
| Perpetual motion | Heartbeat pulse on every active element, noise floor flicker |
| Transducer widgets | Pure-function components: `(state) => visual` |
| CRT materiality | Scanlines, grain, phosphor bleed overlays |
| Light follows significance | `brightness` filter scales with entity importance |

---

## 8. Layout Model — Scrollable Density

**Core principle**: Every page is a single vertically-scrollable container. No viewport-locking. No fixed-height panels that waste space. Content flows naturally top-to-bottom.

**Layout stack** (every page follows this):

```
┌─ TopNav (sticky, always visible) ─────────────────────────┐
│ ⌈ NUNCHI ⌋  │  DEMO  DASH  BENCH  ...  │  ● LIVE 2H 34M  │
├────────────────────────────────────────────────────────────┤
│                                                            │
│  Page Content (overflow-y: auto, scrolls naturally)        │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ Page header (title + subtitle + actions)             │   │
│  ├─────────────────────────────────────────────────────┤   │
│  │ Metric strip (top-level KPIs)                       │   │
│  ├─────────────────────────────────────────────────────┤   │
│  │ Tab bar (if applicable)                             │   │
│  ├─────────────────────────────────────────────────────┤   │
│  │ Content sections (cards, grids, tables, charts)     │   │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐               │   │
│  │  │ Card    │ │ Card    │ │ Card    │               │   │
│  │  └─────────┘ └─────────┘ └─────────┘               │   │
│  │                                                     │   │
│  │  ┌──────────────────────────────────────────────┐   │   │
│  │  │ Full-width section (timeline, chart, etc.)   │   │   │
│  │  └──────────────────────────────────────────────┘   │   │
│  │                                                     │   │
│  │  ┌──────────────┐ ┌────────────────────────────┐   │   │
│  │  │ Sidebar card │ │ Main content card          │   │   │
│  │  └──────────────┘ └────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────┘   │
└────────────────────────────────────────────────────────────┘
```

**Rules:**

1. **Never viewport-lock content.** Pages scroll. The only fixed element is TopNav (sticky).
2. **No empty space.** Every pixel has purpose. If a section is empty, collapse it or show a compact empty indicator.
3. **Dense by default.** Padding is tight: 8-12px inside cards, 8px gaps between cards. Larger gaps (16-24px) only between major sections.
4. **Hierarchical density.** Top-level metrics are large (Fraunces 38px). Section labels are medium (mono 12px). Card content is compact (sans 14px). Detail text is small (mono 11px).
5. **Full-width containers.** Cards and sections stretch to fill available width. Grid layouts use `auto-fill` with small minimums (200px, not 280px).
6. **Content-first sizing.** Container height is determined by content, never by arbitrary min-heights. Exception: charts/canvases with specific aspect ratios.
7. **Scrollable regions inside cards.** When a card has a list or log, it scrolls internally with `max-height` and `overflow-y: auto`, showing a fade gradient at the bottom.

---

## 9. Terminal / Demoscene Aesthetic

**Typography register**: All UI chrome (labels, headings, nav, badges, dividers) uses `var(--mono)` uppercase with letter-spacing. Body text uses `var(--sans)`. Display numbers use `var(--display)` Fraunces italic.

**ASCII vocabulary**:

- Frame brackets: `⌈ LABEL ⌋` for system-level headers, `[ LABEL ]` for interactive elements
- Box-drawing: `─│┌┐└┘├┤┬┴┼` for borders and dividers
- Braille: `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` for loading spinners, `⠀-⣿` range for density fills
- Block elements: `▁▂▃▄▅▆▇█` for inline charts, `░▒▓█` for progress bars
- Status glyphs: `●○◉◐◑ ✓✕ ▸▹ ⬡` for state indicators
- Waveform chars: `▁▂▃▄▅▆▇█` for oscilloscope displays

**NERV institutional register** (from bardo PRD):

- Section headers use mono uppercase, letter-spaced, with optional frame chars
- Status displays use fullwidth or wide-spaced text for gravity
- Metric values are always bone-bright Fraunces, never sans
- Labels are always mono uppercase dim

**Demoscene motion**:

- Scanline overlay on atmospheric layers
- Phosphor decay: bright → dim over ~1s for value changes
- Braille noise floor on decorative surfaces (0.02 opacity, cycles at 10fps)
- Terminal cursor blink on active input fields

---

## 10. Space Efficiency

Concrete token values for density. These override any conflicting values elsewhere in this document.

| Property | Value | Notes |
|----------|-------|-------|
| Card internal padding | 10px 12px | Tight, not airy |
| Gap between sibling cards | 8px | 1px would work for mosaic, 8px for standard |
| Gap between sections | 16px | Sections are visually distinct groups |
| Page horizontal padding | 16px | Edge-to-edge density |
| Metric strip height | auto (content) | No min-height |
| Card border radius | 6px | Slightly less than current 8px for tighter feel |
| Label-to-value gap | 2px | Tight coupling |
| Tab bar padding | 0 | Tabs touch the content below |
| Empty state height | 48px | Compact, not 120px |
| Mosaic cell padding | 16px 14px | Reduced from 30px 28px |

**Anti-patterns** (things NOT to do):

- `min-height: 50vh` on content areas
- Large padding on page containers (>24px)
- Cards with more padding than content
- Empty space below content "for balance"
- Fixed-height sections that don't fill with content

---

## 11. Inference & Cybernetic State System

A unified visual language for inference activity, confidence levels, achievement moments, and collectible artifacts. These subsystems are deeply interconnected: inference tags carry tier colors from 11.1, confidence indicators use progressive intensity from 11.2, threshold crossings trigger crystallization from 11.3, and the resulting artifacts become collectibles from 11.4.

Cross-references: layout context in section 8 (Scrollable Density), typographic register in section 9 (Terminal Aesthetic), and density constraints in section 10 (Space Efficiency). All components here obey the same rules -- they scroll with the page, use tight padding (10-12px), and never viewport-lock.

### New Design Tokens

These extend the `:root` block from section 1.

```css
:root {
  /* ─── Tier spectrum ─── */
  --tier-t0: #8ac4a0;               /* cool/cheap — success-adjacent green */
  --tier-t0-dim: #5a8a6a;           /* ghosted T0 for backgrounds, inactive states */
  --tier-t0-bright: #a8e4c0;        /* highlighted T0 for glow, active states */
  --tier-t1: #c8b890;               /* neutral/standard — reuses --bone */
  --tier-t1-dim: #8a7a5a;           /* ghosted T1 — reuses --bone-dim */
  --tier-t1-bright: #d8c8a0;        /* highlighted T1 — reuses --bone-bright */
  --tier-t2: #cc90a8;               /* warm/expensive — reuses --rose-bright */
  --tier-t2-dim: #7a5060;           /* ghosted T2 — reuses --rose-dim */
  --tier-t2-bright: #dca5bd;        /* highlighted T2 — reuses --rose-glow */

  /* ─── Tier glows ─── */
  --glow-tier-t0: 0 0 12px rgba(138, 196, 160, 0.25);
  --glow-tier-t1: 0 0 12px rgba(200, 184, 144, 0.25);
  --glow-tier-t2: 0 0 12px rgba(204, 144, 168, 0.25);

  /* ─── Intensity spectrum (0.0–1.0 mapped to visual weight) ─── */
  --intensity-ghost: var(--text-ghost);           /* 0.0–0.3 */
  --intensity-building: var(--rose-dim);          /* 0.3–0.6 */
  --intensity-confident: var(--rose);             /* 0.6–0.8 */
  --intensity-crystallized: var(--rose-bright);   /* 0.8–1.0 */

  /* ─── Crystallization ─── */
  --crystal-sparkle: #f0e0f0;
  --crystal-prismatic: linear-gradient(
    135deg,
    rgba(138, 196, 160, 0.08),
    rgba(200, 184, 144, 0.08),
    rgba(204, 144, 168, 0.08),
    rgba(122, 122, 152, 0.08)
  );
  --crystal-ring-color: rgba(220, 165, 189, 0.4);
  --crystal-duration: 800ms;

  /* ─── Cost magnitude ─── */
  --cost-cheap: var(--tier-t0);          /* < $0.001 */
  --cost-moderate: var(--warning);       /* $0.001–$0.01 */
  --cost-expensive: var(--rose-bright);  /* > $0.01 */

  /* ─── Artifact shapes (used with clip-path) ─── */
  --shape-episode: polygon(50% 0%, 100% 25%, 100% 75%, 50% 100%, 0% 75%, 0% 25%);  /* hexagon */
  --shape-insight: polygon(50% 0%, 100% 50%, 50% 100%, 0% 50%);                      /* diamond */
  --shape-hdc: circle(50%);                                                            /* circle */

  /* ─── Slot machine ─── */
  --slot-char-duration: 300ms;
  --slot-char-stagger: 60ms;
  --slot-ease: var(--ease-snappy);
}
```

---

### 11.1 Inference Annotation Tag

Every inference/LLM call in the system carries a visual annotation. This is the universal "inference happened here" marker, used in trace feeds, agent cards, task rows, and bench results. Rendered as a compact horizontal strip of pills/badges following the terminal register (section 9: mono uppercase, letter-spaced).

#### Layout

```
┌──────────────────────────────────────────────────────────────────┐
│ T0  Haiku  anthropic  1.2K→380  $0.0003  142ms                  │
└──────────────────────────────────────────────────────────────────┘
```

Compact inline variant (used inside trace rows, task cells):
```
T1 Sonnet  2.4K→1.1K  $0.008  890ms
```

#### Sub-elements

| Element | Typography | Color | Behavior |
|---------|-----------|-------|----------|
| **Tier badge** | Mono 10px uppercase, `tracking 0.06em` | Background: `var(--tier-tN-dim)`; text: `var(--tier-tN-bright)` | Static after model selection. Pill shape, `border-radius: 3px`, padding `2px 6px`. |
| **Model name** | Mono 11px, `tracking 0.02em` | `var(--tier-tN)` (inherits tier color) | Slot-machine animation on change (see 11.5). Displayed as short name ("Haiku", "Sonnet", "Opus"), not full ID. |
| **Provider** | Mono 10px, `tracking 0.04em` | `var(--text-dim)` | Subtle pill, `border: 1px solid var(--border-soft)`. Only shown in expanded variant, hidden in compact. |
| **Token count** | Mono 11px, tabular-nums | `var(--bone)` for input, `var(--bone-bright)` for output | Format: `{input}→{output}`. Arrow character `→` in `var(--text-ghost)`. Values use `AnimatedNumber` spring on update. |
| **Cost** | Mono 11px, tabular-nums | Color-coded: `var(--cost-cheap)` / `var(--cost-moderate)` / `var(--cost-expensive)` | Dollar prefix, 4 significant digits. Flash animation (section 4.1 `value-flash`) on change. |
| **Latency** | Mono 10px | `var(--text-soft)` | Format: `{n}ms` or `{n.d}s` for >1s. No animation -- updates once on completion. |

#### Sizing (section 10 density compliance)

- Tag height: `22px` (single line, no wasted vertical space)
- Internal padding: `2px 6px` per pill
- Gap between pills: `6px`
- Compact variant omits provider and reduces gap to `4px`
- Full-width variant spaces pills with `justify-content: space-between`

#### Component Interface

```tsx
interface InferenceTagProps {
  tier: 'T0' | 'T1' | 'T2';
  model: string;            // short name: "Haiku", "Sonnet", "Opus"
  provider?: string;        // "Anthropic", "OpenAI" — omit in compact
  inputTokens: number;
  outputTokens: number;
  costUsd: number;
  latencyMs: number;
  variant?: 'full' | 'compact';  // default 'compact'
}
```

#### Tier Color Mapping

| Tier | Meaning | Badge BG | Badge Text | Glow |
|------|---------|----------|------------|------|
| T0 | Cheap/fast (Haiku-class) | `var(--tier-t0-dim)` | `var(--tier-t0-bright)` | `var(--glow-tier-t0)` |
| T1 | Standard (Sonnet-class) | `var(--tier-t1-dim)` | `var(--tier-t1-bright)` | `var(--glow-tier-t1)` |
| T2 | Expensive/capable (Opus-class) | `var(--tier-t2-dim)` | `var(--tier-t2-bright)` | `var(--glow-tier-t2)` |

Note: the existing `06-AGENT-MODEL.md` uses T1/T2/T3 tiers in its agent type definitions. The inference tier here is T0/T1/T2, representing model cost tiers rather than agent routing tiers. The `<Badge variant="tier">` from `09-DESIGN-PRIMITIVES.md` should accept both numbering schemes.

---

### 11.2 Progressive Intensity System

A universal visual language mapping a `0.0`--`1.0` confidence/quality/maturity value to visual weight. This is the system's way of showing "how sure are we?" across every metric surface.

#### Consumers

| Consumer | What the 0.0-1.0 value means |
|----------|------------------------------|
| Cascade router confidence | How confident the router is in its model selection |
| Gate threshold confidence | How stable the adaptive gate threshold has become |
| C-factor quality | Composite quality score for an agent execution |
| Knowledge maturity | How refined/validated a knowledge entry is |
| Episode significance | How important an episode is for learning |
| Somatic marker intensity | Strength of a daimon affect signal |

#### The Four Bands

| Band | Range | Name | Text | Border | Background | Glow | Particles |
|------|-------|------|------|--------|------------|------|-----------|
| Ghost | 0.0--0.3 | Ghost | `var(--text-ghost)`, `opacity: 0.5` | `1px dotted var(--border-soft)` | `transparent` | `none` | None |
| Building | 0.3--0.6 | Building | `var(--text-dim)`, `opacity: 0.8` | `1px solid var(--border)` | `rgba(122, 80, 96, 0.04)` | `none` | None |
| Confident | 0.6--0.8 | Confident | `var(--text-primary)`, `opacity: 1.0` | `1px solid var(--rose-dim)` | `rgba(122, 80, 96, 0.08)` | `0 0 8px var(--rose-dim)` | None |
| Crystallized | 0.8--1.0 | Crystallized | `var(--text-strong)`, `opacity: 1.0` | `1px solid var(--rose)` | `rgba(122, 80, 96, 0.12)` | `var(--glow-rose)` | Sparkle (see 11.3) |

#### CSS Implementation via `data-intensity`

The 0.0--1.0 float maps to a `data-intensity` integer attribute (0--10) for CSS selector targeting. Conversion: `Math.round(value * 10)`.

```css
/* ─── Ghost band (0–3) ─── */
[data-intensity="0"] {
  --intensity-color: var(--text-ghost);
  --intensity-border: 1px dotted var(--border-soft);
  --intensity-bg: transparent;
  --intensity-glow: none;
  --intensity-opacity: 0.4;
}
[data-intensity="1"] {
  --intensity-color: var(--text-ghost);
  --intensity-border: 1px dotted var(--border-soft);
  --intensity-bg: transparent;
  --intensity-glow: none;
  --intensity-opacity: 0.5;
}
[data-intensity="2"] {
  --intensity-color: var(--text-ghost);
  --intensity-border: 1px dotted var(--border);
  --intensity-bg: rgba(122, 80, 96, 0.02);
  --intensity-glow: none;
  --intensity-opacity: 0.6;
}
[data-intensity="3"] {
  --intensity-color: var(--text-dim);
  --intensity-border: 1px dotted var(--border);
  --intensity-bg: rgba(122, 80, 96, 0.03);
  --intensity-glow: none;
  --intensity-opacity: 0.7;
}

/* ─── Building band (4–6) ─── */
[data-intensity="4"] {
  --intensity-color: var(--text-dim);
  --intensity-border: 1px solid var(--border);
  --intensity-bg: rgba(122, 80, 96, 0.04);
  --intensity-glow: none;
  --intensity-opacity: 0.8;
}
[data-intensity="5"] {
  --intensity-color: var(--text-soft);
  --intensity-border: 1px solid var(--border);
  --intensity-bg: rgba(122, 80, 96, 0.05);
  --intensity-glow: 0 0 4px rgba(122, 80, 96, 0.1);
  --intensity-opacity: 0.85;
}
[data-intensity="6"] {
  --intensity-color: var(--rose-dim);
  --intensity-border: 1px solid var(--border-strong);
  --intensity-bg: rgba(122, 80, 96, 0.06);
  --intensity-glow: 0 0 6px var(--rose-dim);
  --intensity-opacity: 0.9;
}

/* ─── Confident band (7–8) ─── */
[data-intensity="7"] {
  --intensity-color: var(--rose);
  --intensity-border: 1px solid var(--rose-dim);
  --intensity-bg: rgba(122, 80, 96, 0.08);
  --intensity-glow: 0 0 8px var(--rose-dim);
  --intensity-opacity: 0.95;
}
[data-intensity="8"] {
  --intensity-color: var(--rose-bright);
  --intensity-border: 1px solid var(--rose);
  --intensity-bg: rgba(122, 80, 96, 0.10);
  --intensity-glow: 0 0 12px var(--rose-dim);
  --intensity-opacity: 1.0;
}

/* ─── Crystallized band (9–10) ─── */
[data-intensity="9"] {
  --intensity-color: var(--rose-bright);
  --intensity-border: 1px solid var(--rose);
  --intensity-bg: rgba(122, 80, 96, 0.12);
  --intensity-glow: 0 0 16px rgba(170, 112, 136, 0.2);
  --intensity-opacity: 1.0;
}
[data-intensity="10"] {
  --intensity-color: var(--rose-glow);
  --intensity-border: 1px solid var(--rose-bright);
  --intensity-bg: rgba(122, 80, 96, 0.14);
  --intensity-glow: var(--glow-rose), 0 0 24px rgba(220, 165, 189, 0.15);
  --intensity-opacity: 1.0;
}
```

#### Applying Intensity

Any element can consume intensity variables:

```css
.intensity-aware {
  color: var(--intensity-color);
  border: var(--intensity-border);
  background: var(--intensity-bg);
  box-shadow: var(--intensity-glow);
  opacity: var(--intensity-opacity);
  transition: color var(--duration-normal) var(--ease-out),
              border-color var(--duration-normal) var(--ease-out),
              background var(--duration-normal) var(--ease-out),
              box-shadow var(--duration-slow) var(--ease-out),
              opacity var(--duration-normal) var(--ease-out);
}
```

Per section 4.2 animation rules: transitions target specific properties (never `transition: all`), and durations stay under 400ms for interactive feedback. The `--duration-slow` (350ms) on `box-shadow` is at the limit -- acceptable because glow changes are ambient, not interactive.

#### Reduced Motion

Under `prefers-reduced-motion: reduce`, disable glow transitions and sparkle particles. Color and opacity transitions remain (they are the non-decorative signal carriers).

```css
@media (prefers-reduced-motion: reduce) {
  .intensity-aware {
    transition: color var(--duration-fast) var(--ease-out),
                opacity var(--duration-fast) var(--ease-out);
    box-shadow: none !important;
  }
}
```

#### Component Interface

```tsx
interface IntensityProps {
  value: number;            // 0.0–1.0
  children: React.ReactNode;
}

// Usage:
<Intensity value={cfactor}>
  <MetricCell label="C-FACTOR" value={cfactor.toFixed(2)} />
</Intensity>

// Renders:
<div data-intensity={Math.round(cfactor * 10)} class="intensity-aware">
  ...
</div>
```

---

### 11.3 Crystallization Effect

The dopamine-reward animation. Triggers when a metric crosses from "confident" into "crystallized" territory (intensity 8+), or on significant discrete achievements. This is the system's way of saying "something meaningful just happened."

#### Trigger Conditions

| Event | Threshold | Where it fires |
|-------|-----------|----------------|
| C-factor crosses 0.8 | `cfactor >= 0.8 && prev_cfactor < 0.8` | Task row, run summary metric |
| Batch gate success | All gates pass in a single task | GateBar component |
| Episode discovery | New episode written to `episodes.jsonl` | Episode feed, artifact counter |
| HDC fingerprint generated | New fingerprint computed | Agent card, knowledge panel |
| Knowledge entry created | New entry persisted to neuro store | Knowledge feed |
| High confidence spike | Router confidence crosses 0.9 | CascadeRouter panel |
| Bench run completes successfully | All tasks pass | Bench run summary |

#### Visual Layers (composited in order)

**Layer 1 -- Sparkle particles (CSS pseudo-elements or canvas)**

```css
@keyframes sparkle-float {
  0% {
    opacity: 1;
    transform: translate(0, 0) scale(1);
  }
  100% {
    opacity: 0;
    transform: translate(
      calc(var(--sparkle-dx) * 1px),
      calc(var(--sparkle-dy) * 1px)
    ) scale(0.3);
  }
}

.crystallize-sparkle {
  position: absolute;
  width: 4px;
  height: 4px;
  background: var(--crystal-sparkle);
  border-radius: 50%;
  pointer-events: none;
  animation: sparkle-float var(--crystal-duration) var(--ease-expo) forwards;
}
```

Spawn 6--12 particles from the element center, with randomized `--sparkle-dx` (-30 to 30) and `--sparkle-dy` (-40 to 10, biased upward). Stagger start by 40--80ms each. Total burst duration: ~800ms. Per section 4.2, this stays within the 8 concurrent CSS animation budget since particles are short-lived.

**Layer 2 -- Prismatic background shimmer**

```css
@keyframes prismatic-shimmer {
  0% {
    background-position: -100% 0;
    opacity: 0;
  }
  30% {
    opacity: 1;
  }
  100% {
    background-position: 200% 0;
    opacity: 0;
  }
}

.crystallize-shimmer {
  position: absolute;
  inset: 0;
  pointer-events: none;
  background: var(--crystal-prismatic);
  background-size: 200% 100%;
  border-radius: inherit;
  animation: prismatic-shimmer 600ms var(--ease-out) forwards;
  mix-blend-mode: screen;
}
```

The gradient sweeps left-to-right through the tier colors (T0 green, T1 bone, T2 rose, plus dream blue), then fades. This references the same palette as the atmospheric layers in section 5.

**Layer 3 -- Ring pulse outward**

```css
@keyframes crystal-ring {
  0% {
    transform: scale(0.8);
    opacity: 0.6;
    border-color: var(--crystal-ring-color);
  }
  100% {
    transform: scale(1.6);
    opacity: 0;
    border-color: transparent;
  }
}

.crystallize-ring {
  position: absolute;
  inset: -4px;
  border: 2px solid var(--crystal-ring-color);
  border-radius: inherit;
  pointer-events: none;
  animation: crystal-ring 500ms var(--ease-expo) forwards;
}
```

Ring expands from 0.8x to 1.6x element size, fading out. Starts slightly inside the element boundary (`inset: -4px`) so the ring origin feels centered.

**Layer 4 -- Sound (optional)**

A subtle achievement chime. Gated behind user preference:

```tsx
const playCrystallize = () => {
  if (userPrefs.sound !== false) {
    const ctx = new AudioContext();
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.type = 'sine';
    osc.frequency.setValueAtTime(880, ctx.currentTime);      // A5
    osc.frequency.exponentialRampToValueAtTime(1760, ctx.currentTime + 0.08); // A6
    gain.gain.setValueAtTime(0.08, ctx.currentTime);
    gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.3);
    osc.connect(gain).connect(ctx.destination);
    osc.start();
    osc.stop(ctx.currentTime + 0.3);
  }
};
```

300ms sine sweep A5 to A6, very quiet (gain 0.08). Never auto-plays on page load -- only on user-initiated actions that result in crystallization events.

#### Composite Timing

```
0ms     Sparkles begin (staggered 40-80ms each)
0ms     Ring pulse begins
50ms    Prismatic shimmer begins (slight delay for layered feel)
500ms   Ring complete
600ms   Shimmer complete
800ms   Last sparkle fades
```

Total: 800ms. Well under the perceptual threshold where animation becomes "slow." Per section 4.2, this fires as a one-shot -- no looping.

#### Debounce

Crystallization effects are debounced per element: maximum one crystallization per 3 seconds per DOM node. Prevents visual noise when multiple rapid threshold crossings occur.

#### Component Interface

```tsx
interface CrystallizeProps {
  trigger: boolean;           // true to fire the effect
  children: React.ReactNode;
  sound?: boolean;            // override user pref
}

// Usage:
<Crystallize trigger={cfactor >= 0.8 && prevCfactor < 0.8}>
  <MetricCell label="C-FACTOR" value="0.84" />
</Crystallize>
```

---

### 11.4 Artifact Collectibles

Episodes, insights, and HDC fingerprints are presented as "collectible artifacts" throughout the UI. Each artifact type has a distinctive visual identity, making them recognizable at a glance in feeds, counters, and detail panels.

#### Artifact Types

| Type | Shape | Clip-path | Base Color | Description |
|------|-------|-----------|------------|-------------|
| Episode | Hexagon | `var(--shape-episode)` | `var(--rose)` | A recorded agent turn -- the atomic unit of learning |
| Insight | Diamond | `var(--shape-insight)` | `var(--bone)` | A distilled finding from episode analysis |
| HDC | Circle | `var(--shape-hdc)` | `var(--dream)` | A hyperdimensional computing fingerprint |

#### Artifact Icon

Small gem/crystal icon (16x16 default, 12x12 compact). Uses `clip-path` for shape, solid fill with subtle inner gradient.

```css
.artifact-icon {
  width: 16px;
  height: 16px;
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.artifact-icon--episode {
  clip-path: var(--shape-episode);
  background: linear-gradient(135deg, var(--rose-dim), var(--rose));
}

.artifact-icon--insight {
  clip-path: var(--shape-insight);
  background: linear-gradient(135deg, var(--bone-dim), var(--bone));
}

.artifact-icon--hdc {
  clip-path: var(--shape-hdc);
  background: linear-gradient(135deg, var(--dream-deep), var(--dream));
}

/* Compact variant for inline use */
.artifact-icon--sm {
  width: 12px;
  height: 12px;
}
```

#### Generation Sparkle

When a new artifact is created (event received via SSE), the icon plays a sparkle entrance:

```css
@keyframes artifact-appear {
  0% {
    transform: scale(0);
    opacity: 0;
    filter: brightness(2);
  }
  60% {
    transform: scale(1.3);
    opacity: 1;
    filter: brightness(1.5);
  }
  100% {
    transform: scale(1);
    opacity: 1;
    filter: brightness(1);
  }
}

.artifact-icon--new {
  animation: artifact-appear 300ms var(--ease-snappy) forwards;
}
```

300ms entrance (per section 4.2 -- under the 400ms cap). Brightness flare sells the "just created" moment.

#### Intensity Integration

Artifact icons participate in the progressive intensity system (11.2). The artifact's significance score maps to intensity, affecting brightness and glow:

```css
.artifact-icon[data-intensity] {
  filter: brightness(calc(0.5 + var(--intensity-opacity) * 0.5));
  box-shadow: var(--intensity-glow);
}
```

Low-significance artifacts are dim; high-significance ones glow.

#### Click Interaction

Clicking any artifact icon opens a detail modal (or expands inline, depending on context). The modal follows the glass panel spec from section 6 (`Pane` component).

#### Counters

Artifact counters accumulate in the TopNav or sidebar. Format follows the terminal register (section 9):

```
⬡ 24   ◇ 8   ● 142
```

- `⬡` (hexagon glyph) for episodes
- `◇` (diamond glyph) for insights
- `●` (circle glyph) for HDC fingerprints
- Count in mono 11px tabular-nums
- Glyph colored by artifact type base color
- Count increments with `AnimatedNumber` spring

Layout: horizontal pill strip in TopNav, `gap: 12px` between counters. Tight enough for section 10 density rules. Total width: ~120px for all three counters.

#### Component Interface

```tsx
interface ArtifactIconProps {
  type: 'episode' | 'insight' | 'hdc';
  intensity?: number;      // 0.0–1.0, drives brightness/glow
  isNew?: boolean;         // triggers appear animation
  size?: 'sm' | 'md';     // 12px or 16px
  onClick?: () => void;    // opens detail view
}

interface ArtifactCounterProps {
  episodes: number;
  insights: number;
  hdcFingerprints: number;
}
```

---

### 11.5 Model Slot Machine

When the cascade router selects a model (or changes model mid-run), the model name display performs a slot-machine character-roll animation. This is the visual equivalent of "the system is making a decision" -- it shows routing is active, not static.

#### Animation Mechanics

Each character in the model name occupies a fixed-width cell (mono font ensures uniform width). Characters "roll" vertically through random characters before settling on the target.

```css
@keyframes slot-roll {
  0% {
    transform: translateY(-100%);
    opacity: 0.3;
  }
  70% {
    transform: translateY(5%);
    opacity: 1;
  }
  100% {
    transform: translateY(0);
    opacity: 1;
  }
}

.slot-char {
  display: inline-block;
  overflow: hidden;
  height: 1.2em;              /* single line height */
  width: 0.6em;               /* mono char width */
  vertical-align: bottom;
  position: relative;
}

.slot-char__inner {
  animation: slot-roll var(--slot-char-duration) var(--slot-ease) forwards;
  animation-delay: calc(var(--char-index) * var(--slot-char-stagger));
}
```

#### Timing

- Per-character settle: `var(--slot-char-duration)` = 300ms
- Character stagger: `var(--slot-char-stagger)` = 60ms (left-to-right cascade)
- Total for "Haiku" (5 chars): 300ms + (4 * 60ms) = 540ms
- Total for "Sonnet" (6 chars): 300ms + (5 * 60ms) = 600ms
- Pre-roll phase: 2--3 random characters cycle at 50ms each before settling

This exceeds the 400ms single-animation cap from section 4.2, but the slot machine is a staggered sequence of sub-400ms animations, not a single long animation. Each character transition is 300ms, which is compliant.

#### Tier Badge Color Shift

When the model changes, the tier badge cross-fades to the new tier color:

```css
.tier-badge {
  transition: background-color var(--duration-normal) var(--ease-out),
              color var(--duration-normal) var(--ease-out),
              box-shadow var(--duration-normal) var(--ease-out);
}
```

The badge color shift happens in parallel with the slot roll, completing ~220ms into the animation. This means the tier color "arrives" before the model name finishes settling -- the user sees the cost tier immediately, then reads the specific model.

#### Pre-roll Characters

During the roll phase, random characters from a curated set cycle through each cell:

```tsx
const SLOT_CHARS = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789'.split('');
```

All uppercase (matching the terminal register from section 9). Characters cycle at 50ms intervals for 2--3 frames before the settle animation begins.

#### Component Interface

```tsx
interface ModelSlotProps {
  model: string;             // target model name: "Haiku", "Sonnet", "Opus"
  tier: 'T0' | 'T1' | 'T2';
  animate?: boolean;         // false to skip animation (initial render)
}
```

On first render, `animate` defaults to `false` -- the model name appears instantly. Animation only plays on subsequent changes.

#### Reduced Motion

Under `prefers-reduced-motion: reduce`, the slot machine degrades to a simple cross-fade (200ms opacity transition). The staggered roll is purely decorative.

```css
@media (prefers-reduced-motion: reduce) {
  .slot-char__inner {
    animation: none;
    transition: opacity var(--duration-normal) var(--ease-out);
  }
}
```

---

### 11.6 Trace Annotations

Every trace/log/event entry in feed components (`<EventStream>`, `<AgentFeed>`, `<InferenceFeed>` from `09-DESIGN-PRIMITIVES.md`) carries inline annotation badges. These annotations use the systems defined in 11.1--11.2 to create a consistent "metadata at a glance" layer across all event types.

#### Annotation Badge Strip

Annotations stack horizontally as compact pills, right-aligned within the trace row. The strip follows the trace row layout from section 8 (scrollable density): tight padding, no wasted space.

```
┌──────────────────────────────────────────────────────────────────────────┐
│ 14:23:07.442  Task gate passed: compile ✓         planner  T1 Sonnet 0.82 │
│               ↑ timestamp       ↑ event text      ↑ agent  ↑ model  ↑ conf│
└──────────────────────────────────────────────────────────────────────────┘
```

#### Badge Types

| Badge | Content | Typography | Color Source | Width |
|-------|---------|-----------|--------------|-------|
| **Agent namespace** | Agent name/role | Mono 10px uppercase, `tracking 0.06em` | Agent identity color (from `lib/palette.ts` `ROLE_COLORS`) | Auto (content-width + `4px 6px` padding) |
| **Model tier + name** | `T1 Sonnet` | Mono 10px | Tier system from 11.1 (`--tier-tN` / `--tier-tN-dim`) | Auto |
| **Confidence** | `0.82` or visual bar | Mono 10px tabular-nums | Progressive intensity from 11.2 (background color from `data-intensity`) | 40px fixed (for alignment) |
| **Cost** | `$0.008` | Mono 10px tabular-nums | Cost magnitude colors from 11.1 | Auto |

#### Row-Level Intensity

The entire trace row's background opacity scales with the event's importance/confidence. This is a subtle application of the progressive intensity system (11.2) at the row level rather than the element level:

```css
.trace-row {
  background: rgba(122, 80, 96, calc(var(--row-importance, 0) * 0.06));
  transition: background var(--duration-normal) var(--ease-out);
}
```

`--row-importance` is a 0.0--1.0 value derived from the event. High-importance rows (gate failures, crystallization events, errors) have a faintly visible rose-tinted background. Low-importance rows (routine inference completions, heartbeats) are transparent. This creates a subtle visual hierarchy within the feed without consuming space -- aligning with section 10's "no empty space" philosophy.

#### Importance Derivation

| Event Type | Base Importance | Modifier |
|------------|----------------|----------|
| Error / gate failure | 0.9 | +0.1 if first failure in sequence |
| Crystallization trigger | 0.8 | -- |
| Episode discovery | 0.7 | +0.1 if high C-factor |
| Gate pass | 0.4 | -- |
| Inference completed | 0.2 | +0.2 if T2 model (expensive = noteworthy) |
| Routine heartbeat | 0.0 | -- |

#### Badge Density

Per section 10 constraints:
- Badge height: 18px (fits within trace row's 22-24px line height)
- Badge internal padding: `2px 6px`
- Badge gap: `4px`
- Badge border-radius: `3px`
- Max badges per row: 4 (agent + model + confidence + cost). If horizontal space is constrained (<600px trace width), confidence and cost collapse into a tooltip on the model badge.

#### Responsive Collapse

At narrow widths, badges collapse progressively:
1. Full: all 4 badges visible
2. Medium (<600px trace width): confidence + cost merge into model badge tooltip
3. Compact (<400px): only agent badge + model tier badge remain
4. Minimal (<300px): agent badge only, colored dot replaces text

This follows the section 8 principle of content-first sizing -- badges never push event text off-screen.

#### Component Interface

```tsx
interface TraceAnnotation {
  agentId?: string;
  agentRole?: string;
  tier?: 'T0' | 'T1' | 'T2';
  model?: string;
  confidence?: number;       // 0.0–1.0
  costUsd?: number;
}

interface TraceRowProps {
  timestamp: number;
  text: string;
  type: string;              // event type for importance derivation
  annotations?: TraceAnnotation;
  children?: React.ReactNode; // expandable detail content
}
```

#### Integration with Existing Feed Components

The trace annotation system overlays onto the feed components specified in `09-DESIGN-PRIMITIVES.md`:

- **`<EventStream>`**: Each event row gains optional `annotations` prop. The existing event reducer (`03-REALTIME-DATA.md` section on event handling) already carries `agent_id`, `model`, `cost_usd` fields -- these map directly to `TraceAnnotation`.
- **`<AgentFeed>`**: Agent namespace badge is always present (the feed is already agent-scoped). Model and confidence badges show per-inference-event.
- **`<InferenceFeed>`**: All annotation fields are present on every row (inference events carry full metadata). This feed is the richest annotation surface.

No new data fetching required -- all annotation data is already present in the SSE event payloads defined in `01-CURRENT-STATE.md` (`InferenceCompleted` carries `model`, `agent_id`, `input_tokens`, `output_tokens`, `cost_usd`, `duration_ms`).

---

### 11.7 Gate Verification Tokens

Design tokens for gate pipeline visualization. Gates are the multi-step validation that runs after each agent task (compile, test, clippy, diff). These tokens enable consistent gate status rendering across `GateBar`, `GateVerdictTicker`, and future `GateVerdictCard` components.

#### New CSS Tokens

These extend the `:root` block from section 1:

```css
:root {
  /* ─── Gate pass ─── */
  --gate-pass: var(--success);                          /* #7a8a78 — sage green */
  --gate-pass-bg: rgba(122, 138, 120, 0.15);           /* Subtle green tint background */
  --gate-pass-border: rgba(122, 138, 120, 0.4);        /* Visible green border */
  --gate-pass-glow: rgba(138, 156, 134, 0.5);          /* Strong glow for flash effect */

  /* ─── Gate fail ─── */
  --gate-fail: var(--rose-bright);                      /* #cc90a8 — warm rose (not harsh red) */
  --gate-fail-bg: rgba(212, 138, 110, 0.12);           /* Subtle warm tint background */
  --gate-fail-border: rgba(212, 138, 110, 0.4);        /* Visible warm border */
  --gate-fail-glow: rgba(212, 138, 110, 0.3);          /* Glow for pulse effect */

  /* ─── Gate running ─── */
  --gate-running: var(--warning);                       /* #c89a68 — warm amber */
  --gate-running-bg: rgba(216, 168, 120, 0.10);        /* Subtle amber tint */
  --gate-running-border: rgba(216, 168, 120, 0.3);     /* Amber border for marching ants */

  /* ─── Gate pending ─── */
  --gate-pending: var(--text-ghost);                    /* #605060 — ghost text */
  --gate-pending-bg: transparent;
  --gate-pending-border: var(--border-soft);            /* Dotted soft border */

  /* ─── Gate skip ─── */
  --gate-skip: var(--text-ghost);
  --gate-skip-bg: transparent;
  --gate-skip-border: var(--border-soft);

  /* ─── Gate chip sizing ─── */
  --gate-chip-height: 28px;
  --gate-chip-padding: 4px 8px;
  --gate-chip-gap: 6px;
  --gate-chip-radius: 4px;
  --gate-group-gap: 12px;
}
```

#### Gate Status Color Mapping

| Status | Icon | Icon color | Background | Border | Animation |
|--------|------|-----------|------------|--------|-----------|
| Pass | `✓` | `var(--gate-pass)` | `var(--gate-pass-bg)` | `var(--gate-pass-border)` | Scale-up 0.95->1.05->1.0 (150ms `--ease-snappy`), brief green glow flash |
| Fail | `✗` | `var(--gate-fail)` | `var(--gate-fail-bg)` | `var(--gate-fail-border)` | Red border pulse (400ms one-shot), icon shake |
| Running | Braille spinner | `var(--gate-running)` | `var(--gate-running-bg)` | `var(--gate-running-border)` | Spinner cycles `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` at 10fps, marching-ants border |
| Pending | `○` | `var(--gate-pending)` | `var(--gate-pending-bg)` | `var(--gate-pending-border)` dotted | No animation, ghost opacity |
| Skipped | `—` | `var(--gate-skip)` | `var(--gate-skip-bg)` | `var(--gate-skip-border)` | Struck-through gate name text |

#### Gate Animation Keyframes

```css
/* Pass flash — brief green glow burst */
@keyframes gate-pass-flash {
  0%   { box-shadow: 0 0 0px var(--gate-pass-glow); }
  40%  { box-shadow: 0 0 12px var(--gate-pass-glow); }
  100% { box-shadow: 0 0 0px transparent; }
}

/* Fail pulse — red border throb */
@keyframes gate-fail-pulse {
  0%   { border-color: var(--gate-fail-border); }
  50%  { border-color: var(--gate-fail-glow); box-shadow: 0 0 8px var(--gate-fail-glow); }
  100% { border-color: var(--gate-fail-border); box-shadow: none; }
}

/* Marching ants — running state border */
@keyframes gate-marching-ants {
  to { background-position: 100% 0; }
}

/* All-pass celebration sweep — green scan line */
@keyframes gate-all-pass-sweep {
  0%   { left: -4px; opacity: 0.6; }
  100% { left: calc(100% + 4px); opacity: 0; }
}
```

#### Connectors Between Gates

When gates are displayed in a horizontal strip (GateVerdictTicker), visual connectors between chips indicate pipeline flow:

| Previous gate status | Connector style |
|---------------------|----------------|
| Pass | Solid 2px line, `var(--gate-pass-border)` |
| Fail | Solid 2px line, `var(--gate-fail-border)` (all subsequent connectors red) |
| Running | Dashed 2px line, `var(--gate-running-border)`, marching-ants animation |
| Pending | Dotted 1px line, `var(--gate-pending-border)` |

#### All-Pass Celebration

When all gates in a task pass, the entire gate strip triggers a coordinated celebration:

1. Each chip gets a brief `gate-pass-flash` animation (staggered 40ms per chip, left-to-right)
2. A green scan line (`4px wide, var(--gate-pass-glow)`) sweeps left-to-right across all chips (200ms)
3. All connectors transition to solid green (100ms)
4. This integrates with the `CrystallizeTransition` (11.3) if wrapping the gate area -- the crystallization fires after the sweep completes

#### Component Cross-References

| Component | File | Uses gate tokens |
|-----------|------|------------------|
| `GateBar` | `src/components/design/GateBar.tsx` | Pass/fail icons and colors |
| `GateVerdictTicker` | `src/components/GateVerdictTicker.tsx` | Full chip rendering with task grouping |
| `GateVerdictCard` (future) | Not yet built | Full card with running state, connectors, celebration |

---

### 11.8 Agent Coordination Tokens

Design tokens for multi-agent handoff and coordination visualization. These tokens are consumed by `AgentHandoff` (section 16 of `10-EXPRESSIVE-PRIMITIVES.md`) and future multi-agent pipeline views.

#### New CSS Tokens

```css
:root {
  /* ─── Handoff particle ─── */
  --handoff-particle-size: 6px;               /* Diamond particle size */
  --handoff-particle-color: var(--rose-bright); /* Base particle color */
  --handoff-flow-duration: 2s;                /* Full particle traversal time */
  --handoff-particle-count: 5;                /* Particles per direction */

  /* ─── Handoff status glows ─── */
  --handoff-active-glow: rgba(216, 154, 178, 0.4);   /* Rose glow during active flow */
  --handoff-done-glow: rgba(122, 138, 120, 0.3);     /* Green glow when handoff complete */
  --handoff-error-glow: rgba(212, 138, 110, 0.3);    /* Warm red glow on error */

  /* ─── Handoff line ─── */
  --handoff-line-width: 2px;
  --handoff-line-active: var(--rose-dim);
  --handoff-line-done: var(--success);
  --handoff-line-pending: var(--border);
  --handoff-line-error: var(--status-error);

  /* ─── Agent node ─── */
  --agent-node-size: 48px;                    /* Default avatar+label container size */
  --agent-node-size-compact: 36px;            /* Compact variant */
  --agent-node-label-size: 11px;              /* Name label font size */
  --agent-node-role-size: 9px;                /* Role sublabel font size */
}
```

#### Handoff Status State Machine

```
  pending ───▶ active ───▶ done
                  │
                  └────▶ error
```

| Status | Line | Particles | Agent nodes | Glow |
|--------|------|-----------|-------------|------|
| `pending` | Dashed, `--handoff-line-pending` | None | Both at idle opacity (0.6) | None |
| `active` | Solid, `--handoff-line-active` | Crystal diamonds flowing along line | Source node "working" glow, target node "idle" | `var(--handoff-active-glow)` on line |
| `done` | Solid, `--handoff-line-done` | None (particles settle and fade) | Both nodes at full opacity, target shows "done" | `var(--handoff-done-glow)` residual, fades over 1s |
| `error` | Dashed, `--handoff-line-error` | None (particles freeze and red-shift) | Source node error state | `var(--handoff-error-glow)` pulse |

#### Particle Animation

```css
/* Crystal particle flowing along handoff line */
@keyframes handoff-flow-forward {
  0%   { left: 0%; opacity: 0; }
  10%  { opacity: 0.8; }
  90%  { opacity: 0.8; }
  100% { left: 100%; opacity: 0; }
}

@keyframes handoff-flow-reverse {
  0%   { right: 0%; opacity: 0; }
  10%  { opacity: 0.8; }
  90%  { opacity: 0.8; }
  100% { right: 100%; opacity: 0; }
}

.agent-handoff__particle {
  position: absolute;
  width: var(--handoff-particle-size);
  height: var(--handoff-particle-size);
  background: var(--handoff-particle-color);
  transform: rotate(45deg);               /* Diamond shape from rotated square */
  opacity: 0;
  pointer-events: none;
}

.agent-handoff__particle--forward {
  animation: handoff-flow-forward var(--handoff-flow-duration) linear infinite;
}

.agent-handoff__particle--reverse {
  animation: handoff-flow-reverse var(--handoff-flow-duration) linear infinite;
}
```

Each of the 5 particles is staggered by `flow-duration / particle-count` (400ms) via `animation-delay`.

#### Direction Modes

| Direction | Forward particles | Reverse particles | Arrow placement |
|-----------|------------------|-------------------|----------------|
| `forward` | Yes (5) | No | Right end (▶) |
| `reverse` | No | Yes (5) | Left end (◀) |
| `bidirectional` | Yes (5) | Yes (5) | Both ends (◀▶) |

#### Reduced Motion

Under `prefers-reduced-motion: reduce`, no particle animation. Active state shows a solid gradient line (left color = source agent color, right color = target agent color) instead of flowing particles. Done state shows a solid green line.

#### Component Cross-References

| Component | File | Uses coordination tokens |
|-----------|------|------------------------|
| `AgentHandoff` | `src/components/agent/AgentHandoff.tsx` | Full handoff visualization |
| `AgentContainer` | `src/components/agent/AgentContainer.tsx` | Agent node styling |
| `AgentAvatar` | `src/components/agent/AgentAvatar.tsx` | Agent identity within nodes |

---

### 11.9 Terminal Configuration

Design specifications for the xterm.js terminal integration. The terminal is a primary interaction surface in the demo app -- agent output, command execution, and scenario progress all render here. These specifications ensure terminal rendering is consistent with the ROSEDUST design system.

#### xterm.js Option Set

Full option configuration applied at Terminal construction:

| Option | Value | Token/rationale |
|--------|-------|-----------------|
| `theme` | `rosedustTheme` | Custom 16-color palette from `src/lib/rosedust-theme.ts` |
| `fontFamily` | `'JetBrainsMono Nerd Font Mono', 'JetBrains Mono', 'SF Mono', monospace` | Matches `var(--mono)` with Nerd Font priority for powerline glyphs |
| `fontSize` | `12` | Denser than default; matches section 10 density principles |
| `lineHeight` | `1.1` | Tight (default 1.2); aligns with card content density targets |
| `letterSpacing` | `0` | Mono font handles spacing; no extra needed |
| `cursorStyle` | `'bar'` | Modern feel; less intrusive than block cursor |
| `cursorWidth` | `2` | Visible but not heavy; `--rose-bright` color from theme |
| `cursorBlink` | `true` | Standard terminal behavior |
| `cursorInactiveStyle` | `'outline'` | Hollow rectangle when terminal unfocused |
| `scrollback` | `5000` | Generous for reviewing agent output history |
| `smoothScrollDuration` | `80` | Subtle smoothing; not jarring jumps |
| `drawBoldTextInBrightColors` | `false` | Prevents bright ANSI bold from being unreadable |
| `fontWeight` | `'400'` | Normal body weight |
| `fontWeightBold` | `'600'` | Semi-bold, not full bold |
| `minimumContrastRatio` | `1` | Disabled; trust ROSEDUST theme colors |
| `overviewRulerWidth` | `8` | Thin scrollbar minimap on right edge |
| `customGlyphs` | `true` | Box-drawing and powerline glyph renderer |
| `allowProposedApi` | `true` | Future API access |

#### ROSEDUST Terminal Palette

The terminal palette maps ANSI colors to ROSEDUST spectrum values. Every color avoids harsh primaries -- reds are terracotta, greens are sage, blues are lavender. This ensures `cargo test`, `git diff`, and agent ANSI output blend with the surrounding UI.

| ANSI slot | Color | Hex | ROSEDUST mapping |
|-----------|-------|-----|-----------------|
| background | Deep void | `#0c0a10` | Darker than `--bg-void` for depth contrast |
| foreground | Muted rose-grey | `#c4b4c4` | Adjacent to `--text-soft` |
| cursor | Rose | `#d89ab2` | Within `--rose-bright` spectrum |
| selection | Rose tint | `rgba(184, 122, 148, 0.28)` | Semi-transparent rose for readability |
| black | Deep void | `#18141e` | |
| red | Terracotta | `#d48a6e` | Warm, not alarming |
| green | Sage | `#8a9c86` | Adjacent to `--success` |
| yellow | Amber | `#d8a878` | Adjacent to `--warning` |
| blue | Lavender | `#8888a8` | Adjacent to `--dream` |
| magenta | Rose | `#d89ab2` | Core rose spectrum |
| cyan | Teal | `#6a9ea0` | Adjacent to `--status-active` |
| white | Warm bone | `#e4d8b0` | Adjacent to `--bone-bright` |
| brightBlack | Rose grey | `#443844` | Rose-tinted neutral |
| brightRed | Light terracotta | `#e8a088` | |
| brightGreen | Light sage | `#a4c4a0` | |
| brightYellow | Light amber | `#e8c090` | |
| brightBlue | Light lavender | `#a4a4c8` | |
| brightMagenta | Light rose | `#e8b5ce` | |
| brightCyan | Light teal | `#8abcbe` | |
| brightWhite | Cream | `#f0e4d0` | Adjacent to `--text-strong` |

#### Terminal Chrome Tokens

CSS tokens for terminal pane chrome (scrollbar, focus ring, header bar):

```css
/* ─── Terminal scrollbar ─── */
--terminal-scrollbar-width: 6px;
--terminal-scrollbar-thumb: rgba(168, 112, 140, 0.3);
--terminal-scrollbar-thumb-hover: rgba(168, 112, 140, 0.5);
--terminal-scrollbar-track: transparent;

/* ─── Terminal focus ─── */
--terminal-focus-ring: 0 0 0 1px rgba(220, 165, 189, 0.2);

/* ─── Terminal header density ─── */
--terminal-header-padding: 2px 10px;
--terminal-header-label-size: 10px;
--terminal-header-status-size: 9px;
--terminal-header-dot-size: 5px;
```

#### File Cross-References

| What | File |
|------|------|
| Terminal hook (xterm config) | `src/hooks/useTerminal.ts` |
| ROSEDUST theme definition | `src/lib/rosedust-theme.ts` |
| Terminal pane CSS overrides | `src/components/Terminal/TerminalPane.css` |
| Terminal header density | `src/pages/Demo.css` |
