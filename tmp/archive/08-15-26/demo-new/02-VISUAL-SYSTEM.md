# Visual System — ROSEDUST for Web Demos

Adapted from `bardo/prd/18-interfaces/rendering/00-design-system.md`.
Simplified for web (CSS) rather than TUI (ratatui).

## CSS Custom Properties

```css
:root {
  /* ── Foundation ───────────────────────────── */
  --bg-void:      #060608;   /* Deep violet-black */
  --bg-raised:    #0C0A0E;   /* Panels, containers */
  --bg-mid:       #080810;   /* Headers, status bars */
  --bg-warm:      #0A0808;   /* Warm-shifted for degraded states */

  /* ── Rose Spectrum (primary) ──────────────── */
  --rose:         #AA7088;   /* Primary text, headers, active data */
  --rose-bright:  #CC90A8;   /* Alerts, emphasis */
  --rose-dim:     #7A5060;   /* Secondary labels */
  --rose-deep:    #3A2030;   /* Background tints, ghost text */
  --rose-ember:   #482838;   /* Phosphor residue, borders */

  /* ── Bone (single emphasis — use sparingly) ─ */
  --bone:         #C8B890;   /* Max ONE element per screen */
  --bone-dim:     #8A7A5A;   /* Dimmed emphasis */

  /* ── Text Hierarchy ────────────────────────── */
  --text:         #988090;   /* Standard readable text */
  --text-dim:     #584858;   /* Secondary labels */
  --text-ghost:   #302830;   /* Barely visible */

  /* ── Semantic ──────────────────────────────── */
  --dream:        #585878;   /* Indigo, dream state */
  --sage:         #70887A;   /* Success, pass */
  --ember:        #AA8855;   /* Warning, amber */
  --fail:         #C36E55;   /* Error, red-amber */

  /* ── Glass ─────────────────────────────────── */
  --glass-bg:     rgba(255,255,255,0.02);
  --glass-border: rgba(255,255,255,0.05);
  --glass-blur:   12px;

  --glass-2-bg:     rgba(255,255,255,0.04);
  --glass-2-border: rgba(255,255,255,0.08);
  --glass-2-blur:   20px;

  /* ── Layout ────────────────────────────────── */
  --radius:       6px;
  --radius-sm:    3px;
  --gap:          8px;
  --panel-width:  280px;

  /* ── Animation ─────────────────────────────── */
  --ease-luxury:  cubic-bezier(0.22, 1, 0.36, 1);
  --ease-subtle:  cubic-bezier(0.25, 0.46, 0.45, 0.94);
  --transition:   200ms var(--ease-luxury);
}
```

## Typography

```css
/* Font stack */
body {
  font-family: 'JetBrains Mono', 'Geist Mono', 'SF Mono', 'Menlo', monospace;
  font-size: 13px;
  line-height: 1.4;
  color: var(--text);
  background: var(--bg-void);
}

/* Hierarchy */
.display  { font-size: 1.5rem; color: var(--rose); font-weight: 500; }
.heading  { font-size: 0.85rem; color: var(--rose-dim); text-transform: uppercase;
            letter-spacing: 0.1em; }
.label    { font-size: 0.7rem; color: var(--text-dim); }
.value    { font-size: 0.85rem; color: var(--text); font-variant-numeric: tabular-nums; }
.emphasis { color: var(--bone); }  /* Use max once per view */
```

## Glass Panels

```css
.glass {
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
}

.glass-2 {
  background: var(--glass-2-bg);
  border: 1px solid var(--glass-2-border);
  border-radius: var(--radius);
  backdrop-filter: blur(var(--glass-2-blur));
}
```

## xterm.js Theme

```javascript
const ROSEDUST_XTERM = {
  background:          '#060608',
  foreground:          '#988090',
  cursor:              '#AA7088',
  cursorAccent:        '#060608',
  selectionBackground: '#3A2030',
  selectionForeground: '#C8B890',
  black:               '#0C0A0E',
  red:                 '#C36E55',
  green:               '#70887A',
  yellow:              '#AA8855',
  blue:                '#585878',
  magenta:             '#AA7088',
  cyan:                '#6A9EA0',
  white:               '#988090',
  brightBlack:         '#3A2030',
  brightRed:           '#D48A6E',
  brightGreen:         '#99C4AC',
  brightYellow:        '#D9B870',
  brightBlue:          '#9D98C4',
  brightMagenta:       '#CC90A8',
  brightCyan:          '#8ABCBE',
  brightWhite:         '#C8B890',
};
```

## Component Patterns

### Metric Card
```html
<div class="metric glass">
  <span class="label">total cost</span>
  <span class="value">$0.042</span>
</div>
```

### Gate Indicator
```html
<div class="gate pass">
  <span class="gate-icon">✔</span>
  <span class="gate-name">compile</span>
</div>
<!-- States: .pending (dim), .pass (sage), .fail (fail) -->
```

### File Entry
```html
<div class="file new">
  <span class="file-icon">🦀</span>
  <span class="file-name">src/main.rs</span>
</div>
<!-- .new = sage color + flash animation on creation -->
```

### Scenario Tab
```html
<button class="tab active" data-scenario="self-hosting">
  Self-Hosting
</button>
<!-- .active = rose underline, text becomes --rose -->
```

### Cost Ticker (digit-slide animation)
```css
.cost-ticker {
  font-variant-numeric: tabular-nums;
  color: var(--bone);
  font-size: 0.85rem;
}

.cost-ticker .digit {
  display: inline-block;
  overflow: hidden;
  height: 1.2em;
}

.cost-ticker .digit-inner {
  transition: transform 300ms var(--ease-luxury);
}
```

## Layout Grid

```css
/* Terminal grid — 1, 2, or 4 panes */
.terminal-grid {
  display: grid;
  gap: var(--gap);
  flex: 1;
  overflow: hidden;
}

.terminal-grid.cols-1 { grid-template-columns: 1fr; }
.terminal-grid.cols-2 { grid-template-columns: 1fr 1fr; }
.terminal-grid.cols-4 { grid-template-columns: 1fr 1fr; grid-template-rows: 1fr 1fr; }
```

## Color Rules (from the 7 Rendering Laws)

1. **80% rose dominance** — Interface is monochromatic-primary
2. **Bone appears once per screen** — Only for the single most important metric
3. **Never pure #000 or #FFF** — Use `--bg-void` and `--bone` instead
4. **Sage = success, Ember = warning, Fail = error** — No other semantic colors
5. **Glass for containers** — Not solid backgrounds
6. **Restrained animation** — `var(--ease-luxury)`, 200ms default, no bouncing
7. **50%+ empty space** — Negative space is intentional
