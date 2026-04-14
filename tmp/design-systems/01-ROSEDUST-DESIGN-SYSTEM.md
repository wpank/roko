# ROSEDUST Design System — Complete Reference

Use this document as a design system bible when generating dark, atmospheric, cinematic web experiences. Every token, every value, every craft detail is canonical.

---

## Identity

**Name:** ROSEDUST v2
**Vibe:** Terminal existentialism. Rose light on violet-black, seen through dirty CRT glass. Not cyberpunk, not vaporwave — crafted, atmospheric, alive.
**Aesthetic DNA:** Serial Experiments Lain × NieR: Automata × Evangelion NERV × James Turrell × Rothko

---

## Color Palette

### Backgrounds
```css
--bg-void:        #060608;      /* Primary — cool-rose tinted dark, NOT pure gray */
--bg-raised:      #0a0810;      /* Elevated surface — ~3% OKLCH lightness apart */
--bg-mid:         #080810;
--bg-deeper:      #040406;
--bg-glass:       rgba(8, 8, 12, 0.45);
--bg-glass-hover: rgba(58, 32, 48, 0.14);
--bg-glass-active:rgba(58, 32, 48, 0.32);
```

### Borders
```css
--border:         rgba(255, 255, 255, 0.07);   /* Use rgba white — adapts to any bg */
--border-soft:    rgba(255, 255, 255, 0.04);
--border-strong:  rgba(255, 255, 255, 0.14);
--border-active:  var(--rose-glow);
```

### Rose Spectrum — Primary Accent
```css
--rose:           #aa7088;
--rose-bright:    #cc90a8;
--rose-glow:      #dca5bd;       /* The signature color — used for emphasis, headings, glows */
--rose-dim:       #7a5060;
--rose-deep:      #3a2030;       /* Background tints, hover states */
--rose-ember:     #482838;
```

### Bone Spectrum — Secondary Accent (Value/Cost/Provenance)
```css
--bone:           #c8b890;
--bone-bright:    #d8c8a0;       /* Used for metrics, values, the "important number" */
--bone-dim:       #8a7a5a;
```

### Text
```css
--text-primary:   #c8b8c0;      /* NOT pure white — tinted off-white, ~90% opacity equivalent */
--text-strong:    #d8c8d0;      /* Maximum text brightness — never use #fff */
--text-soft:      #988090;
--text-dim:       #6a5a68;       /* Labels, secondary info */
--text-ghost:     #3a303a;       /* Placeholders, pending states */
```

### Semantic
```css
--dream:          #7a7a98;       /* Knowledge, learning, background context */
--dream-bright:   #9494b4;
--dream-deep:     #282848;
--success:        #7a8a78;       /* Gate pass, healthy, connected */
--warning:        #c89a68;       /* Degraded, retrying, attention */
--danger:         #cc5555;       /* Crash, fail — used sparingly */
```

### State Colors
```
Active/running:    --rose-glow
Value/cost/money:  --bone-bright
Passing/healthy:   --success
Warning/attention: --warning
Failed/error:      --rose-bright
Pending/inactive:  --text-dim
Info/neutral:      --dream-bright
```

### Glow Effects
```css
--glow-active:    0 0 12px rgba(45, 212, 191, 0.3);
--glow-success:   0 0 12px rgba(74, 222, 128, 0.3);
--glow-error:     0 0 12px rgba(251, 113, 133, 0.3);
--glow-ambient:   0 0 20px rgba(255, 255, 255, 0.05);
--glow-rose:      0 0 20px rgba(170, 112, 136, 0.15);
```

### Selection
```css
::selection { background: var(--rose-deep); color: var(--rose-glow); }
```

---

## Typography

### Font Families
```css
--mono:    "JetBrains Mono", ui-monospace, monospace;
--display: "Fraunces", "Times New Roman", serif;
```

Google Fonts import:
```
Fraunces:ital,opsz,wght@0,9..144,300;0,9..144,400;0,9..144,500;1,9..144,300;1,9..144,400;1,9..144,500
JetBrains+Mono:wght@300;400;500
```

### Type Scale
| Role | Family | Weight | Size | Tracking | Line-height |
|------|--------|--------|------|----------|-------------|
| Display heading | Fraunces italic | 300 | clamp(46px, 5.6vw, 82px) | -0.022em | 1.02 |
| Page hero (h1) | Fraunces | 300 | clamp(72px, 9vw, 140px) | -0.025em | 0.94 |
| Section heading | Fraunces italic | 400 | 30px | -0.012em | 1.18 |
| Body | Fraunces | 300 | 19px | +0.005em | 1.62 |
| Body standard | Fraunces | 400 | 16px | +0.005em | 1.7 |
| Label | JetBrains Mono | 400 | 11px | +0.28em | uppercase |
| Label small | JetBrains Mono | 400 | 10px | +0.22em | uppercase |
| Mono value | JetBrains Mono | 400 | 14px | +0.02em | — |
| Metric large | Fraunces italic | 400 | 38px | -0.015em | 1 |

### Letter-Spacing Rules
```
32px+:      -0.02em to -0.03em   (pull in — large type needs tightening)
20-30px:    -0.01em              (barely perceptible)
14-16px:     0 to +0.01em        (default tracking)
11-13px:    +0.02em to +0.04em   (open up — small type needs air)
UPPERCASE:  +0.06em to +0.10em   (caps always need significant tracking)
```

### Emphasis Patterns
- **Headings `<em>`**: rose-glow italic with text-shadow glow: `text-shadow: 0 0 24px rgba(204,144,168,.45)`
- **Body `<em>`**: rose-glow italic, subtler glow: `text-shadow: 0 0 12px rgba(204,144,168,.22)`
- **`.bone` class**: bone-bright, normal style — for value/metric callouts
- **`<b>` in body**: text-strong, weight 400 — subtle emphasis without bold

---

## Shadows & Elevation

```css
--shadow-sm:  0 1px 3px rgba(0,0,0,0.4), inset 0 1px 0 rgba(255,255,255,0.06);
--shadow-md:  0 4px 16px rgba(0,0,0,0.5), inset 0 1px 0 rgba(255,255,255,0.06);
--shadow-lg:  0 12px 40px rgba(0,0,0,0.6);
--shadow-glow-rose: 0 0 0 1px rgba(220,165,189,0.3), 0 0 20px rgba(170,112,136,0.15);
```

**Critical craft detail:** Every elevated surface gets `inset 0 1px 0 rgba(255,255,255,0.06)` — a specular top-edge highlight simulating light from above. Removing it makes panels feel flat.

---

## Glassmorphism

```css
backdrop-filter: blur(12px) saturate(180%);
background: var(--bg-glass);
border: 1px solid var(--border);
```

The `saturate(180%)` is critical — without it, glass panels look dull. This is how macOS menu bars achieve their premium feel.

**TopNav glass:** `backdrop-filter: blur(16px) saturate(180%)`, `background: rgba(6,6,8,0.85)`

---

## Motion Design

### Easing
```css
--ease-snappy: cubic-bezier(0.2, 0.8, 0.2, 1);    /* Vercel's standard */
--ease-expo:   cubic-bezier(0.16, 1, 0.3, 1);
--ease-out:    cubic-bezier(0, 0, 0.2, 1);
```

**Rule:** Use `ease-out`, never `ease` or `ease-in-out` for hover effects. `ease` has a slow start that introduces perceived latency.

### Duration
```css
--duration-instant: 80ms;     /* color/opacity changes */
--duration-fast:    150ms;    /* border, transform, hover */
--duration-normal:  220ms;    /* tooltip, dropdown, panel */
--duration-slow:    350ms;    /* page transition, modal */
```

### Keyframes

```css
/* LED pulse — status dots */
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}
/* 2.4s ease-in-out infinite */

/* Element entrance */
@keyframes fadeUp {
  from { opacity: 0; transform: translateY(12px); }
}
/* 200ms var(--ease-expo) forwards, stagger: calc(var(--i) * 40ms) */

/* CRT flicker — barely perceptible */
@keyframes flicker {
  0%, 98% { opacity: 1; }
  99% { opacity: 0.97; }
}
/* 11s linear infinite */

/* Loading shimmer */
@keyframes shimmer {
  0%   { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}
/* 1.8s ease-in-out infinite */

/* Value change highlight */
@keyframes value-flash {
  0%   { color: var(--bone-bright); text-shadow: 0 0 8px rgba(216,200,160,0.3); }
  100% { color: inherit; text-shadow: none; }
}
/* 300ms var(--ease-out) */
```

### Craft Details — What Separates Good from World-Class

1. **Specular top-edge highlight** on every elevated surface: `inset 0 1px 0 rgba(255,255,255,0.06)`
2. **rgba white borders** (not hex): `rgba(255,255,255,0.07)` adapts to any background
3. **Hover transforms are small**: `translateY(-2px)`, not 8px. `scale(1.005)`, not 1.05
4. **Asymmetric press timing**: 50ms on press (snappy), 120ms on release (smooth)
5. **Staggered list entrance**: 40ms between items — fast enough to not feel slow
6. **Skeleton shimmer at dark-UI range**: 0.03-0.07 opacity (not consumer 0.1-0.2)
7. **Tooltip entrance combines opacity AND transform**: 4px Y travel + 0.97 scale
8. **Glass panels use saturate()**: `backdrop-filter: blur(12px) saturate(180%)`
9. **Never `transition: all`**: always list specific properties
10. **`will-change: transform`** on hover-animating elements
11. **All UI feedback under 400ms** — 500ms+ reads as broken

---

## Atmospheric Layers

Three fixed-position overlays, `pointer-events: none`, that give the interface a crafted/physical feel:

### 1. Grain Texture (most important — 80% of the effect)
```css
.grain {
  position: fixed; inset: 0; pointer-events: none; z-index: 9997;
  opacity: 0.035; mix-blend-mode: overlay;
  background-image: url("data:image/svg+xml;utf8,<svg xmlns='...' width='240' height='240'><filter id='n'><feTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='3' stitchTiles='stitch'/></filter><rect width='100%25' height='100%25' filter='url(%23n)'/></svg>");
}
```

### 2. Scanlines
```css
body::after {
  content: ""; position: fixed; inset: 0; pointer-events: none; z-index: 9999;
  background: repeating-linear-gradient(to bottom, transparent 0, transparent 2px,
    rgba(0,0,0,0.45) 2px, rgba(0,0,0,0.45) 3px);
  opacity: 0.06; mix-blend-mode: multiply;
}
```

### 3. Vignette
```css
body::before {
  content: ""; position: fixed; inset: 0; pointer-events: none; z-index: 9998;
  background: radial-gradient(ellipse at 50% 30%, transparent 50%, rgba(6,6,8,0.72) 100%);
}
```

---

## Component Patterns

### Section Tag (`.stag`)
```css
font-family: var(--mono); font-size: 11px; letter-spacing: 0.32em;
text-transform: uppercase; color: var(--text-dim); font-weight: 400;
```
Prefix: `—— ` in rose-dim. Pattern: `—— 01 · LABEL`

### Pane (Glass Panel)
- Left rose border: `2px solid var(--rose-dim)` with glow
- Background: `var(--bg-glass)` with `1px var(--border)` border
- Header: mono 10.5px uppercase tracking 0.06em, dim text
- Specular highlight on body
- LED prop: 5px glowing dot before label

### LED Dot
```css
width: 5px; height: 5px; border-radius: 50%;
background: var(--success);
box-shadow: 0 0 6px var(--success);
animation: pulse 2.2s infinite;
```
Three-layer glow: tight 1px ring at 40% + 8px soft outer. The tight ring makes it look engineered.

### Mosaic (Metric Grid)
- Gap: 1px with `var(--border)` showing through
- Cell: padding 30px 28px, bg `var(--bg-glass)`
- Label: mono 10px uppercase, dim
- Value: Fraunces italic 400, 38px, bone-bright
- 40ms stagger per cell on mount

### Button
```css
display: inline-flex; align-items: center; gap: 14px;
padding: 14px 26px; border: 1px solid var(--rose-dim);
color: var(--rose-glow); font-family: var(--mono);
font-size: 11px; letter-spacing: 0.22em; text-transform: uppercase;
background: rgba(58,32,48,0.1); transition: all 0.25s;
```
Hover: `background: var(--rose-deep); border-color: var(--rose-glow); box-shadow: 0 0 22px rgba(204,144,168,0.22)`

### Axiom (Pull Quote)
```css
text-align: center; max-width: 1000px; margin: 90px auto;
```
- Label: mono 10px, rose-dim, tracking 0.32em
- Quote: Fraunces italic 300, clamp(28px, 3.4vw, 46px), text-strong
- `<em>` in quote: rose-glow with text-shadow

### Table
- Full-width, mono 13px
- Headers: mono 10px uppercase, dim, tracking 0.28em
- First column: Fraunces italic 16px, text-strong (named entities)
- Row hover: `var(--bg-glass-hover)`, 80ms transition
- 40ms stagger per row on mount

---

## Spacing
```css
--gap-xs:   4px;
--gap-sm:   8px;
--gap-md:   16px;
--gap-lg:   24px;
--gap-xl:   40px;
--gap-2xl:  64px;
--wrap:     1240px;
--gutter:   64px;
--section-y: 200px;
```

**No border-radius.** ROSEDUST uses sharp corners exclusively.

---

## Icon System
```
● filled  = done
◉ ring    = active
○ hollow  = pending
✓         = pass
✕         = fail
```
Always pair with text label — never color alone.

---

## Focus State
```css
:focus-visible {
  outline: none;
  box-shadow: 0 0 0 2px var(--bg-void), 0 0 0 4px rgba(220,165,189,0.7);
}
```
Double-ring: dark inner gap (2px) + rose outer ring (4px).

---

## Responsive
```css
@media (max-width: 1100px) { :root { --gutter: 36px; --section-y: 130px; } }
@media (max-width: 760px)  { :root { --gutter: 22px; --section-y: 90px; } }
```

---

## Inspirations
- **Vercel** (Rauno Freiberg): horizontal nav, high density, specular highlights, `cubic-bezier(.2,.8,.2,1)`
- **Linear**: high-density developer UI, keyboard-first
- **Stripe**: premium dark UI patterns
- **NieR: Automata**: cinematic title screens, naming ceremonies
- **Evangelion NERV**: institutional register, amber/green/cyan/red status
- **Serial Experiments Lain**: dissociation, interface-within-interface
- **James Turrell / Rothko**: sub-perceptual color shifts, immersion
- **Eve Online**: ambient particle fields, glow effects, dark theme accents
