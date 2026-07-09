# Site Generation Mega-Prompt

Use this as a complete prompt (or split into sections) when asking an LLM to generate a full Three.js / WebGL interactive documentation site.

---

## PROMPT

You are building a single-file HTML page that serves as an interactive documentation site / product showcase. The site uses the **ROSEDUST** design system and features multiple Three.js / WebGL scenes with interactive elements.

### Technical Stack
- Single HTML file, self-contained
- Three.js 0.160.0 via ES module importmap: `{ "imports": { "three": "https://unpkg.com/three@0.160.0/build/three.module.js" } }`
- Google Fonts: Fraunces (display/body, ital+opsz, weights 300/400/500) + JetBrains Mono (labels/code, weights 300/400/500)
- Pure CSS — no frameworks. Responsive breakpoints at 1100px and 760px
- ES modules in `<script type="module">` blocks
- No build step, no npm, no React

### Core Design System

**Color palette (CSS variables on `:root`):**
```
Backgrounds: --bg-void:#060608, --bg-raised:#0a0810, --bg-mid:#080810, --bg-deeper:#040406
Borders: --border:#1a1622 (or rgba(255,255,255,0.07)), --border-soft:#13111b
Rose spectrum: --rose:#aa7088, --rose-bright:#cc90a8, --rose-glow:#dca5bd, --rose-dim:#7a5060, --rose-deep:#3a2030, --rose-ember:#482838
Bone spectrum: --bone:#c8b890, --bone-bright:#d8c8a0, --bone-dim:#8a7a5a
Text: --text-primary:#c8b8c0, --text-strong:#d8c8d0, --text-soft:#988090, --text-dim:#6a5a68, --text-ghost:#3a303a
Semantic: --dream:#7a7a98, --dream-bright:#9494b4, --warning:#c89a68, --success:#7a8a78
```

**Typography:**
```
--mono: "JetBrains Mono", ui-monospace, monospace
--display: "Fraunces", "Times New Roman", serif
```
- h1: Fraunces 300, clamp(72px,9vw,140px), tracking -0.025em, line-height 0.94
- h2: Fraunces 300, clamp(46px,5.6vw,82px), tracking -0.022em
- h3: Fraunces 400, 30px, tracking -0.012em
- Body: Fraunces 300, 19px, line-height 1.62
- Labels: JetBrains Mono 400, 11px, tracking 0.28em, uppercase
- All `<em>` tags in headings: `color: var(--rose-glow); text-shadow: 0 0 24px rgba(204,144,168,0.45)`
- Never use pure white (#fff) — max brightness is --text-strong (#d8c8d0)

**Atmospheric layers (MANDATORY — these make it feel crafted):**
1. Grain overlay: `position:fixed; opacity:0.035; mix-blend-mode:overlay` using SVG feTurbulence noise
2. Scanlines: `repeating-linear-gradient` at `opacity:0.06`
3. Vignette: `radial-gradient(ellipse, transparent 50%, rgba(6,6,8,0.72) 100%)`
4. CRT flicker: `@keyframes flicker { 0%,98%{opacity:1} 99%{opacity:0.97} }` at 11s period

**Craft details that MUST be present:**
- Every elevated surface: `inset 0 1px 0 rgba(255,255,255,0.06)` specular highlight
- Glass panels: `backdrop-filter: blur(12px) saturate(180%)`
- Hover transforms: `translateY(-2px)` maximum, never 8px
- List/card entrances: staggered fadeUp with 40ms delay per item
- Buttons: mono 11px uppercase, rose border, rose-glow text, rose-deep hover bg with glow
- Section tags: `—— 01 · LABEL` pattern, mono 11px, tracking 0.32em
- No border-radius anywhere — all sharp corners
- Selection color: `::selection { background: var(--rose-deep); color: var(--rose-glow) }`

### Page Structure

**Loading curtain:** Full-screen overlay with rotating diamond shape, fades out after 2-3s.

**TopNav:** Fixed, glass background, diamond brand mark, centered navigation links, "OPEN APP" button with rose border. Right side: live status dot.

**Scroll progress bar:** 1.5px fixed bar at top with rose gradient fill tracking scroll %.

**Hero section:** Full viewport height.
- Three.js canvas filling the entire section as background
- Particle swarm scene: 200+ octahedra orbiting/swarming in rose/bone/dream colors
- Optional: state toggle buttons (chaos ↔ coordination) that smoothly transition particle formations
- Content overlay (z-index above canvas): centered text with staggered entrance animation
  - Pre-label in mono capsule with rose border
  - h1 with 3 lines, each animating in with 0.3s delay
  - Subtitle in Fraunces 300 21px
  - CTA buttons
  - Quote tag at bottom
- Scroll indicator at bottom: "EXPLORE" with bouncing arrow

**Content sections:** Each section follows the pattern:
```html
<section>
  <div class="wrap">
    <div class="stag"><span class="num">01</span><span class="label">SECTION NAME</span></div>
    <h2>Section heading with <em>italic rose accent</em></h2>
    <p class="lead">Lead paragraph...</p>
    <!-- Content: panes, mosaics, tables, interactive visualizations -->
  </div>
</section>
```

**Interactive Three.js panels:** Canvas wrapped in `.cwrap` containers with HUD overlays at corners showing labels and values. Radial gradient background behind canvas. Vignette overlay on top.

**Pane component:** Glass panel with header bar (mono 10.5px label, LED dots, status text) and body. Left rose border accent. Specular highlight.

**Mosaic component:** Grid with 1px gaps, cells containing label/value pairs. Values in Fraunces italic bone-bright.

**Axiom component:** Centered pull quote with label, large italic quote, and corollary text.

**Interactive grids:** Clickable tiles that update a detail panel. Active tile: rose-glow border + rose-deep background + inset glow.

**Tables:** Full-width mono, first column in Fraunces italic for named entities, dim headers, hover highlight.

**Terminal demo:** Simulated CLI with:
- macOS-style title bar (three colored dots)
- Subtle scanlines in terminal body
- Typed commands with prompt character (❯) in rose
- Success (✓ green), info (◇ rose), warning (⚠ amber) line prefixes
- Blinking cursor with rose glow

**Telemetry sidebar:** Fixed right-edge panel (hidden <1280px) with live-updating system metrics, values in Fraunces italic with glow.

**Hover definition terms:** `<span class="defterm" data-def="explanation">term</span>` — dashed underline, tooltip on hover with rose left border and shadow.

**Outro section:** Centered closing statement, large italic type with rose accents, CTA buttons, footer line.

### Three.js Scene Patterns

All scenes use `MeshBasicMaterial` (no lighting needed). Key geometry: `OctahedronGeometry` for crystalline shapes. Always include:
- Root group with slow rotation: `root.rotation.y = t * 0.08`
- `InstancedMesh` for many similar objects (one draw call)
- Dust particles: `Points` with `AdditiveBlending` and `depthWrite: false`
- Wireframe + solid layering for depth (solid core + wireframe rim + transparent halo)
- Ring geometry for orbital paths
- Line geometry for spokes/connections
- Smooth transitions via exponential approach: `x += (target - x) * 0.03`

**ROSEDUST Three.js colors:**
```javascript
const ROSE = 0xaa7088, ROSE_GLOW = 0xdca5bd, ROSE_DIM = 0x7a5060;
const BONE = 0xd8c8a0, DREAM = 0x7a7a98, SUCCESS = 0x7a8a78;
```

### Animation System

**Entrance animations:**
- Hero h1 lines: 1.4s cubic-bezier(0.16,0.7,0.18,1), staggered 0.3s
- Cards/panes: fadeUp 200ms with 40ms stagger
- Terminal lines: 250ms slide-in from left

**Hover states:**
- Cards: translateY(-2px) + box-shadow-md, 200ms
- Buttons: rose-deep bg + rose-glow border + 22px glow shadow
- Table rows: bg-glass-hover, 80ms
- Interactive tiles: border-color + bg shift, 200ms

**Ongoing animations:**
- LED pulse: 2.4s ease-in-out infinite
- CRT flicker: 11s linear infinite
- Scroll progress: JS-driven width transition
- Telemetry values: periodic updates with value-flash animation
- Three.js scenes: continuous 60fps rendering

### Quality Checklist

Before considering the page complete:
- [ ] Grain, scanlines, vignette atmospheric layers present
- [ ] No pure white text anywhere
- [ ] All elevated surfaces have specular top-edge highlight
- [ ] Glass panels use `saturate(180%)` in backdrop-filter
- [ ] All hover transforms are ≤2px
- [ ] Staggered entrances on lists/grids
- [ ] Loading curtain with diamond pulse animation
- [ ] Scroll progress bar functional
- [ ] At least one interactive Three.js scene
- [ ] Terminal demo with realistic styling
- [ ] Responsive at 1100px and 760px breakpoints
- [ ] Selection color is rose-deep/rose-glow
- [ ] No border-radius anywhere
- [ ] All labels are mono uppercase with wide tracking

---

## TOPIC-SPECIFIC PROMPT ADDONS

### For an Agent/AI Product Site

Add these content sections:
1. **The Problem** — dramatic quote, four-element grid explaining the bottleneck
2. **Control Plane / Architecture** — Three.js visualization of system topology (nodes + connections + data flow)
3. **CLI Demo** — Terminal simulation showing real commands and output
4. **Primitives** — Table of core concepts with numbers, names, descriptions, properties
5. **Protocols / Patterns** — Interactive grid with radar or orrery visualization
6. **Cost Comparison** — Side-by-side cost breakdown panels
7. **Use Cases** — 2×2 grid of personas and scenarios
8. **Why Now** — Three tailwinds/trends with statistics

### For a Technical Documentation Site

Add these sections:
1. **Architecture Overview** — Interactive diagram (Three.js) showing system components
2. **Getting Started** — Terminal demo with install/setup commands
3. **API Reference** — Expandable table of endpoints/methods
4. **Configuration** — Pane with code examples and parameter descriptions
5. **Benchmarks** — Mosaic grid with performance metrics, interactive chart

### For a Design System Showcase

Add these sections:
1. **Color Palette** — Visual swatches with hex values and usage notes
2. **Typography Scale** — Live examples at each size/weight
3. **Component Gallery** — Interactive demos of each component (pane, mosaic, button, etc.)
4. **Animation Library** — Triggerable animation demos
5. **Three.js Scenes** — Gallery of available background scenes

---

## STYLE REFERENCE SNIPPETS

### Minimal Page Skeleton
```html
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>Title · Subtitle</title>
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@300;400;500&family=Fraunces:ital,opsz,wght@0,9..144,300;0,9..144,400;0,9..144,500;1,9..144,300;1,9..144,400;1,9..144,500&display=swap" rel="stylesheet" />
<script type="importmap">
{ "imports": { "three": "https://unpkg.com/three@0.160.0/build/three.module.js" } }
</script>
<style>
:root {
  --bg-void:#060608; --bg-raised:#0a0810; --bg-deeper:#040406;
  --border:#1a1622; --border-soft:#13111b;
  --rose:#aa7088; --rose-bright:#cc90a8; --rose-glow:#dca5bd;
  --rose-dim:#7a5060; --rose-deep:#3a2030; --rose-ember:#482838;
  --bone:#c8b890; --bone-bright:#d8c8a0; --bone-dim:#8a7a5a;
  --text-primary:#c8b8c0; --text-strong:#d8c8d0; --text-soft:#988090;
  --text-dim:#6a5a68; --text-ghost:#3a303a;
  --dream:#7a7a98; --dream-bright:#9494b4;
  --warning:#c89a68; --success:#7a8a78;
  --mono:"JetBrains Mono",ui-monospace,monospace;
  --display:"Fraunces","Times New Roman",serif;
  --wrap:1240px; --gutter:64px; --section-y:200px;
}
/* ... full CSS here ... */
</style>
</head>
<body class="crt">
<div class="grain"></div>
<!-- ... content ... -->
<script type="module">
import * as THREE from "three";
// ... Three.js scenes ...
</script>
</body>
</html>
```
