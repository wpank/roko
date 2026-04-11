# Demo Visual Specification: Nunchi Series A

**Purpose**: Detailed visual design specification for the Nunchi demo — what every screen looks like, the design system, UI patterns, typography, color, animation, and competitive visual benchmarks. Written for someone with zero prior context.

**Date**: April 2026

---

## 1. Design System Overview

The demo has two surfaces: a **CLI terminal** (the primary demo artifact — what investors see during the pitch) and a **web dashboard** (the secondary artifact — shown during technical diligence or as background during conversation). Both surfaces must feel like the same product.

### The Design Challenge

The current React dashboard uses a design system called ROSEDUST: dark purple-black backgrounds, dusty rose/pink accents, serif display font (Playfair Display), monospace code font. This aesthetic is warm and atmospheric but does not match the competitive benchmark set by Linear, Vercel, and Temporal. For investor-facing demos, the design must signal:

- **Technical credibility** (this is infrastructure, not a consumer product)
- **Taste** (the founders care about craft — a leading indicator of product quality)
- **Confidence** (minimal chrome, data speaks for itself)

### The Recommendation: Evolve, Don't Abandon

ROSEDUST's dark-first approach is correct for the audience (developers view on desktop, dark mode is genre convention — 8 of 12 agent-infrastructure sites surveyed use dark by default). But the accent palette and typography need updating to match the competitive tier.

**What to keep**: Dark backgrounds, monospace-heavy data displays, minimal chrome, atmospheric depth.

**What to change**: Replace dusty rose accent with Vercel-style blue for primary actions. Replace Playfair Display serif with Geist Sans for display typography. Tighten spacing. Increase contrast. Add elevation through luminance, not color overlays.

---

## 2. Terminal Design (CLI Demo Surface)

The terminal is the primary demo surface. What investors see is a terminal running `nunchi` commands. This must look premium.

### Font

**Primary recommendation**: Berkeley Mono ($75 license, TX-02 by Neil Pachal at U.S. Graphics) at 24-28pt for all terminal output. Coalesces 1970s machine-readable typefaces with humanist qualities. 150+ ligatures. 5 widths, 12 weights. Used by Tobi Lütke and Brendan Dolan-Gavitt. Signals "I read the manual; I have taste" more distinctly than any other choice. **Typography is signaling, not decoration.**

Reasoning against Geist: Geist reads as "I deploy to Vercel" — the signal is slightly taken. Inter reads as efficient, not aesthetic. Berkeley Mono reads as infrastructure-native. The body font should be a Söhne-grade neutral grotesk or similar.

**Eccentric option worth considering**: Ship a custom "Nunchi Mono" typeface, OFL-licensed (like Vercel did with Geist). Instantly recognizable in every screenshot ever taken of a Roko terminal. High effort but maximal brand signal.

**Fallback**: JetBrains Mono — best free all-purpose coding font. 139 ligatures, 8 weights, thorough hinting. Renders excellently on non-retina displays.

**Color accent constraint**: Avoid green (Supabase owns it) and orange (Replit/HN). The accent blue (#4A9EFF or Tokyo Night blue #7AA2F7) is safe territory.

### Terminal Theme

**Tokyo Night** palette on `#1A1B26` background. This is locked for all demo surfaces.

```
Background:       #1A1B26
Active Background: #1F2335
Line Highlight:    #292E42
Foreground:        #C0CAF5
Comment:           #565F89
Blue:              #7AA2F7
Purple:            #BB9AF7
Cyan:              #7DCFFF
Red:               #F7768E
Green:             #9ECE6A
Yellow:            #E0AF68
```

Why Tokyo Night: The electric blue/purple palette evokes sophistication without being cold. Strong community association with Neovim and modern CLI tools. Catppuccin Mocha (the other top choice) is softer/warmer but reads as less technical. Tokyo Night is the right signal for infrastructure.

### Terminal Emulator

**Ghostty**. Pre-configured with Tokyo Night theme, Geist Mono font, font size locked before the meeting. Ghostty prioritizes native GPU rendering — no frame drops during the demo.

### CLI Output Symbols

Clack-style symbols. No emoji. Ever.

```
◆  Section header (filled diamond)
◇  Sub-section (open diamond)
│  Vertical connector
└  Final item in list
✔  Pass / success (green)
✖  Fail / error (red)
⚠  Warning (yellow)
ℹ  Info (blue)
❯  Prompt / input
→  Flow / transition
⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏  Dots spinner (Braille)
```

### What the Terminal Output Looks Like

This is the exact visual the investor sees. Every line carries a primitive.

```
$ nunchi run agents/researcher.py --task "Summarize Q3 fintech earnings"

◆ Agent
│  researcher@v2  ·  nhi://acme/researcher.v2  (✔ verified)
│
◇ Predict
│  $0.043  ·  12.4s  ·  route: haiku → gpt-4o-mini
│
◇ Gates
│  ✔ pii_scan       ✔ cost_ceiling<$0.10    ✔ sox_compliance
│
◇ Knowledge
│  loaded 7 facts from /finance/q3  (3 agents, 0.91 avg conf)
│
◇ Running
│  ████████████████████████████████████  done in 9.8s
│
◇ Result
│  ✔ $0.031 actual  (-28% vs predicted)  ·  routed to haiku
│  → deposited 2 new facts → /finance/q3
│
└ Share: https://nunchi.network/runs/abc123
```

Key visual properties:
- The progress bar uses Unicode block elements (█), not ASCII
- Colors: agent line in cyan, predict in yellow, gates in green, knowledge in purple, cost delta in green (favorable), share URL in blue underline
- The vertical line (│) creates visual continuity down the output
- Numbers are the largest visual elements — they're what investors remember

### Scripted Keystrokes

Use `demo-magic` for reproducible keystroke playback during the critical path. No live typing. The typing should look human-speed (not instant, not glacially slow). Mistakes and backspaces can be pre-scripted to look natural.

### Recording

**VHS** (charmbracelet/vhs) for pre-recorded backup. Declarative `.tape` file:

```
Set FontSize 24
Set Width 1200
Set Height 600
Set Theme "Tokyo Night"
Set FontFamily "Geist Mono"

Type "nunchi run agents/researcher.py --task 'Summarize Q3 fintech earnings'"
Enter
Sleep 2s
# ... output appears
```

Output: MP4 for Keynote fallback, GIF for deck appendix.

**Asciinema** for interactive recordings where text should be selectable/copyable.

---

## 3. Web Dashboard Design (Secondary Demo Surface)

The web dashboard is shown during technical diligence or runs on a second screen during the pitch conversation. It must make data feel alive.

### Design Tokens (Evolved from ROSEDUST)

```css
/* ---- Backgrounds (luminance-based elevation) ---- */
--bg-base:          #0F0F0F;     /* Page background — near-black, not pure black */
--bg-raised:        #1A1A1A;     /* Cards, panels — 5-8% luminance increase */
--bg-overlay:       #252525;     /* Modals, tooltips */
--bg-hover:         #303030;     /* Hover states */
--bg-active:        #3A3A3A;     /* Active/selected states */

/* ---- Text (off-white prevents glare) ---- */
--text-primary:     #E5E5E5;     /* Body text — reads as "white" without glare */
--text-secondary:   #A0A0A0;     /* Secondary labels */
--text-tertiary:    #6C6C6C;     /* Hints, timestamps */
--text-disabled:    #4A4A4A;     /* Disabled elements */

/* ---- Accent (Vercel-style blue for primary actions) ---- */
--accent:           #4A9EFF;     /* Primary accent — buttons, links, active tabs */
--accent-dim:       #2563EB;     /* Accent on hover */
--accent-bg:        rgba(74, 158, 255, 0.10);  /* Accent background tint */

/* ---- Semantic colors ---- */
--success:          #9ECE6A;     /* Pass, success, healthy (Tokyo Night green) */
--success-bg:       rgba(158, 206, 106, 0.10);
--fail:             #F7768E;     /* Fail, error, critical (Tokyo Night red) */
--fail-bg:          rgba(247, 118, 142, 0.10);
--warn:             #E0AF68;     /* Warning, caution (Tokyo Night yellow) */
--warn-bg:          rgba(224, 175, 104, 0.10);
--info:             #7AA2F7;     /* Info, neutral highlight (Tokyo Night blue) */
--info-bg:          rgba(122, 162, 247, 0.10);
--purple:           #BB9AF7;     /* Knowledge, identity (Tokyo Night purple) */
--purple-bg:        rgba(187, 154, 247, 0.10);

/* ---- Borders ---- */
--border-subtle:    rgba(255, 255, 255, 0.06);
--border-default:   rgba(255, 255, 255, 0.12);
--border-strong:    rgba(255, 255, 255, 0.20);

/* ---- Typography ---- */
--font-sans:        'Geist Sans', 'Inter', -apple-system, sans-serif;
--font-mono:        'Geist Mono', 'JetBrains Mono', 'SF Mono', monospace;

/* ---- Spacing ---- */
--space-xs:         4px;
--space-sm:         8px;
--space-md:         16px;
--space-lg:         24px;
--space-xl:         32px;
--space-2xl:        48px;

/* ---- Radius ---- */
--radius-sm:        4px;
--radius-md:        8px;
--radius-lg:        12px;

/* ---- Transitions ---- */
--ease:             cubic-bezier(0.4, 0, 0.2, 1);
--duration-fast:    150ms;
--duration-normal:  200ms;
--duration-slow:    300ms;
```

### Typography Scale

```
Display:    Geist Sans, 48px, weight 500, letter-spacing -0.02em
Headline:   Geist Sans, 32px, weight 500, letter-spacing -0.01em
Title:      Geist Sans, 24px, weight 500
Subtitle:   Geist Sans, 18px, weight 500
Body:       Geist Sans, 14px, weight 400, line-height 1.5
Small:      Geist Sans, 12px, weight 400, line-height 1.4
Mono:       Geist Mono, 13px, weight 400, line-height 1.6
Mono-sm:    Geist Mono, 11px, weight 400, line-height 1.5
Data:       Geist Mono, 28px, weight 500  (for stat cards / big numbers)
```

Key rules:
- Medium weight (500) reads cleaner than regular (400) on dark backgrounds at small sizes
- Slightly increased letter-spacing (-0.02em to -0.01em) on display sizes
- Body text line-height 1.5 for comfortable reading
- Monospace for all data, numbers, code, and technical labels

### Surface Elevation

Shadows don't work on dark backgrounds. Use luminance-based elevation instead:

```
Surface 0:  #0F0F0F  (page background)
Surface 1:  #1A1A1A  (cards, panels, sidebar)
Surface 2:  #252525  (modals, tooltips, dropdowns)
Surface 3:  #303030  (hover states within cards)
```

Each level is a 5-8% luminance increase. Borders at each level: `rgba(255, 255, 255, 0.06)` to `rgba(255, 255, 255, 0.12)`.

---

## 4. The Four Dashboard Views

The existing dashboard has 7 pages with 27+ sections. Too busy for a demo. Strip to four views that tell the whole story. Everything else is accessible behind a secondary nav.

### View 1: Cost Dashboard (The "Proof It Works" View)

This is the landing view. The first thing investors see when the dashboard loads.

**Layout**: Full-width header with 4 stat cards, then a main content area with a large real-time cost chart on the left (70% width) and a routing decisions panel on the right (30% width).

**Stat cards** (horizontal row, equal width):

```
┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
│  Total Cost  │ │  Cache Hit   │ │  Routing     │ │  Gate Pass   │
│              │ │              │ │              │ │              │
│   $1.42      │ │   65%        │ │   82% cheap  │ │   94%        │
│   ▼ 30x      │ │   ▲ from 7%  │ │   haiku/mini │ │   7-rung     │
└─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘
```

Each stat card:
- Large monospace number (28px, Geist Mono, weight 500)
- Muted label above (12px, text-secondary)
- Delta indicator below (green for favorable, red for unfavorable)
- Subtle color coding: cost=warn, cache=info, routing=purple, gate=success

**Cost chart** (Canvas 2D, not a charting library):
- Cumulative cost over time, line chart
- Two lines: "naive baseline" (dashed, red-tinted) and "Nunchi optimized" (solid, accent blue)
- The gap between them is the savings — shaded in accent-bg
- Y-axis: dollar amount. X-axis: time or task number
- Real-time updates: new data points animate in with a soft fade (200ms ease-out)
- Hover tooltip shows: task name, model used, cost, cache hit status, routing decision

**Routing decisions panel** (right side):
- Vertical list of recent routing decisions
- Each entry: task snippet (truncated), model selected, cost, confidence score
- Color-coded by model tier: green for cheap (haiku/mini), yellow for mid (sonnet), red for expensive (opus)
- Scrolling with most recent at top

### View 2: Agent Fleet (The "It's Real" View)

**Layout**: Grid of agent cards, 2-3 per row depending on screen width.

**Each agent card**:

```
┌──────────────────────────────────────┐
│  ◆ researcher@v2                     │
│  nhi://acme/researcher.v2  ✔         │
│                                      │
│  ┌────┬────┬────┬────┬────┬────┬───┐ │
│  │CODE│RSRC│LTCY│COST│SAFE│GATE│CLB│ │
│  │ 94 │ 87 │ 91 │ 96 │ 99 │ 93 │ 88│ │
│  └────┴────┴────┴────┴────┴────┴───┘ │
│                                      │
│  Current: "Summarize Q3 earnings"    │
│  Status: running ████░░░░ 62%        │
│  Cost: $0.019 / $0.043 predicted     │
│  Uptime: 4h 23m  ·  Tasks: 47       │
└──────────────────────────────────────┘
```

Key visual elements:
- Agent identity line with verification badge
- 7-domain reputation scores displayed as a compact horizontal bar (one cell per domain, color-coded by score: green >90, yellow 70-90, red <70)
- Current task with progress bar
- Cost meter showing actual vs predicted
- The reputation bar is the distinctive visual — no other tool has on-chain agent reputation

### View 3: Knowledge Graph (The "Network Effect" View)

**Layout**: Full-width visualization. Primary recommendation is a terrain/contour map; force-directed graph is retained as an alternative.

#### Primary Recommendation: Terrain (d3-contour + Canvas 2D)

Compounding maps to elevation (peaks rise as confidence accumulates); demurrage maps to erosion (peaks shrink, valleys widen). Topographic conventions are universal — no legend needed. `d3-contour` ships `contourDensity().weight()` doing this exactly. ~200 LOC delta from current d3-force code.

**Implementation core**:

```js
const N = 128;
const grid = new Float32Array(N*N);
const contours = d3.contours().size([N,N]).thresholds(12);
const polys = contours(grid);
const color = d3.scaleSequential([0, max], d3.interpolateViridis);
```

Position via UMAP/t-SNE of node embeddings (deterministic). Per-frame KDE on 128-squared grid + 12 thresholds = ~3-6ms on M1 = 60fps.

**Visual design (terrain)**:
- Dark background (#0F0F0F)
- Viridis color ramp: low-confidence valleys are deep blue/purple, high-confidence peaks are yellow/green
- Contour lines render as thin strokes at each threshold — reads like a topographic map
- Domain clusters emerge as separate peaks (finance peak, code peak, research peak)
- Demurrage decay visually erodes peaks over time — terrain literally sinks without new citations
- New knowledge publications raise local elevation with a radial pulse animation
- Hover on a contour region: show top facts, contributing agents, aggregate confidence, decay rate

**The story this tells**: Knowledge compounds into peaks. The terrain rises where agents invest attention and erodes where they don't. The investor watches peaks form in real time — a landscape shaped by collective agent intelligence.

#### Strong Secondary: Mycelial/Physarum (Landing Page Hero Background)

Same algorithm NASA used to map dark matter. 200k agents on 1024-squared trail map at 60fps on integrated GPU. Paths reinforce themselves — visually embodies compounding. Best used as a living background texture on the landing page hero section rather than the dashboard view.

#### Tertiary Fallback: Star Map

`d3-celestial` (BSD-3 licensed). Magnitude scale maps to decay — bright stars are high-confidence facts, dim stars are decaying. Familiar celestial convention requires no explanation.

#### Alternative: Force-Directed Graph (d3-force)

**Implementation**: Canvas 2D with d3-force layout (not WebGL — unnecessary complexity for this use case). Alternatively react-force-graph for React integration.

**Visual design**:
- Dark background (#0F0F0F)
- Knowledge nodes as circles, sized by citation frequency (more cited = larger)
- Node color by domain (finance = warm amber, code = blue, research = purple)
- Citation edges as thin lines with directional arrows
- Edge opacity proportional to citation frequency (more cited = more opaque)
- Nodes in demurrage decay shown at reduced opacity (fading)
- New knowledge publications animate in with a ripple effect
- Hover on a node: show content summary, source agent, creation date, citation count, decay status

**The story this tells**: Knowledge compounds. The graph gets denser over time. New agents connect to existing knowledge immediately. The network effect is visible.

**What to avoid**: Don't make it look like a blockchain explorer (nodes as blocks with hashes). Make it look like a knowledge graph — organic, growing, alive.

### The Pulse Globe (Agent Topology Cold-Open)

Engineering decision: `three-globe` (vasturiano, MIT) in a custom Three.js scene with EffectComposer (UnrealBloomPass). Reference: janarosmonaliev/github-globe (MIT). 60fps at 5,000 simultaneous arcs on M1/M2.

**Color palette for Lens categories** (designed for #05060a background):

| Lens | Hex | Use |
|---|---|---|
| planner | #7C5CFF (Nunchi violet) | Long-running planner agents |
| executor | #22D3EE (cyan) | Tool/code-exec sub-agents |
| retriever | #34D399 (mint) | Search/RAG agents |
| critic | #F472B6 (magenta-pink) | Eval/review agents |
| human | #FBBF24 (amber) | Human-in-the-loop events |
| system | #94A3B8 (slate) | Bus/control plane events |

**Key parameters**:
- Particle size: 2.5-4.0px
- Arc dash animate: 1500ms
- Atmosphere color: #7C5CFF
- UnrealBloomPass strength: 0.9
- Slider binds to `emissionsPerSecond` in [0, 200] AND drives BusBridge throughput (real load knob)

**Hero animation**: Full-bleed mount, auto-rotate 0.3 deg/s, pause on mouseenter, viewport-tiered emission, DPR `Math.min(2, devicePixelRatio)`, <400kB gz total.

### View 4: Chain View (The "Coordination Plane" View)

**Layout**: Split-screen. Left: live block feed. Right: summary stats.

**Left panel — Block feed** (scrolling, newest at top):

```
Block 1,204,387  ·  50ms  ·  3 txns
├─ Knowledge published: /finance/q3 (researcher@v2)
├─ Identity attestation: analyst@v1 (renewed)
└─ ZK proof verified: HDC similarity 0.91

Block 1,204,386  ·  48ms  ·  2 txns
├─ Knowledge published: /finance/q3 (analyst@v1)
└─ Reputation update: researcher@v2 (+0.02 code_quality)
```

Each block entry:
- Block number, time, transaction count
- Indented transaction summaries with type icons
- Color-coded by type: knowledge=purple, identity=blue, proof=green, reputation=amber

**Right panel — Summary stats**:
- Active agents (number with live count)
- Knowledge entries (total, with growth rate)
- ZK proofs verified (total, with recent rate)
- Average block time (with sparkline showing consistency)
- Network health indicator (green dot, pulsing)

---

## 5. Demo Mode

A "Demo Mode" toggle in the top bar that transforms the dashboard for investor meetings.

**When activated**:
- Hides secondary navigation (only the 4 demo views visible)
- Auto-cycles through views on a 45-second timer
- Shows a discreet timer in the corner: "Next view in 38s"
- Keyboard shortcut: press `D` to activate/deactivate
- Press `1`, `2`, `3`, `4` to jump to specific views
- Press `Space` to pause/resume auto-cycling

**Visual difference in demo mode**:
- Stat card numbers are 20% larger
- Animations are slightly more pronounced (300ms instead of 200ms)
- A subtle "LIVE" indicator pulses in the top-right corner
- The Nunchi wordmark is visible in the bottom-left

---

## 6. The Shareable URL Page (`nunchi.network/runs/<id>`)

This is the artifact that leaves the room with the investor. It must be impressive on its own.

**Layout**: Single-page, no navigation needed. Dark background matching the dashboard tokens.

**Header**:
```
nunchi.network/runs/abc123
Agent: researcher@v2  ·  nhi://acme/researcher.v2  (verified)
Task: "Fix the failing test in src/auth.rs"
Status: ✔ Completed  ·  Cost: $0.14  ·  Duration: 8.2s
```

**Main content — Execution Timeline**:
- Vertical timeline with steps on the left, detail panel on the right
- Each step shows: step number, description, model used, cost, duration, gate result
- Click any step to see: full prompt, response, tool calls, routing decision, gate evaluation
- The timeline is the hero visual — it encodes the entire agent run as a navigable, auditable artifact

**Cost breakdown bar**:
- Horizontal stacked bar showing cost composition: cache savings (gray), routing savings (blue), gate savings (green), actual spend (accent)
- Label: "This run cost $0.14. Naive baseline: $4.18. Reduction: 30x."

**ZK Proof section**:
- Proof hash (truncated with copy button)
- Block number and timestamp
- "Verify on-chain" link
- Simple explanation: "This proof attests that this run's results match the claimed computation, verified without revealing the underlying data."

**Footer**:
- "Powered by Nunchi — the Agent Coordination Plane"
- Link to docs, link to GitHub

### Execution Trace Implementation

The strongest converging signal from visualization research: Chrome DevTools and Datadog independently land on the same pattern — multi-track, time-aligned, canvas-rendered, hover-synced. Elements to steal:

| Element | Source | Implementation |
|---|---|---|
| Multiple synchronized tracks sharing one x-axis | Chrome DevTools Performance | Canvas2D + React overlay; tracks for agent execution flame, $/sec ribbon, token throughput, latency p99 |
| Per-agent categorical color palette (hash-to-HSL) | Jaeger UI | One hue per agent; eye instantly reads system shape |
| Mini-map context strip at top | Jaeger | CSS transform for spans (not SVG rect) for perf |
| Drag-select to outlier histogram | Honeycomb BubbleUp | D3 + canvas heatmap |
| Shade-by-cost ramp (lightness encodes $) | Datadog Trace View 2023 | hue=agent, lightness=cost; expensive spans glow |
| Log/event to timeline hover sync | Datadog | Vertical cursor on flame graph synced to cost line items |
| Animated dashed "marching ants" for in-flight | Temporal | CSS @keyframes on stroke-dasharray; best "this is live" signal |
| vis-timeline as base library | Temporal | https://github.com/visjs/vis-timeline |
| Inline $ cost labels on every bar | LangSmith | Most agent traces hide cost; Nunchi must show it |
| Power-user command palette (Ctrl+Shift+P) | Perfetto | Optional; doesn't clutter casual viewers |

### Computation Receipt Mockups

Four visual language options for the ZK proof artifact (the thing the investor takes home):

**Mockup A — "Notarized Page"**: Paper-cream + ink-black + wax-seal. 720px-wide A4-portrait card. Fonts: EB Garamond 28pt small-caps headings, Source Serif 4 15/24 body, JetBrains Mono 13 hashes. Palette: paper-cream #FAF6EE, ink #1A1A1A, hairline #C9C2B2, wax-red #7E1F1F, apostille-gold #A8895C. SVG-generated guilloche border, centered wax-seal SVG. Feel: Sotheby's lot certificate.

**Mockup B — "Etherscan Receipt"**: Dark + neon. Bg #0B0E14, panel #11151D, text #E6EAF2, magenta #FF61D2, green #3FB950, cyan #58A6FF. Inter + IBM Plex Mono. Top status pill + property/value table + expandable logs accordion. Pulsing cube icon next to proof hash.

**Mockup C — "Lab Notebook"**: Lined paper + signed/dated two-page spread. Fonts: Caveat for "Witnessed by," iA Writer Quattro body. Palette: notebook-cream #F5EFE0, line-blue #A8B5C8, ink-blue #1B2A4E. Deterministic SVG identicon from run hash, red rubber-stamp graphic at 7-degree rotation. Feel: Newton's notebook.

**Mockup D — Hybrid (RECOMMENDED DEFAULT)**: Dark canvas (Mockup B) for execution timeline + cost; embedded paper-cream Mockup A "receipt card" floats lower-right at ~340x480px with drop-shadow. Click flips card (3D CSS `rotateY(180deg)`) to show technical proof internals. Card is a downloadable PDF. **Why this wins:** contrast between active dark canvas and still cream card visually encodes the pitch — "we run agents at scale and produce a permanent artifact." The card is the literal "thing that leaves the room."

### Cost Comparison Visualization Patterns

Three patterns for the $0.14 vs $4.18 (~30x) visualization:

**Pattern 1 — "Crushed Bar" (RECOMMENDED HERO)**: Two horizontal bars stacked. Top: $4.18 Naive at 100% width, hot-red #E5484D with scan-line texture. Bottom: $0.14 Nunchi at 3.3% width, muted-green #3FB950. Dotted vertical at 3.3% labeled "30x less." Pure HTML/CSS. On-viewport-enter animation: green bar draws 0 to 3.3% over 800ms with `cubic-bezier(0.2, 0.8, 0.2, 1)`. **Why it wins at 30x:** linear bars are the sweet spot at this ratio. Tufte-correct.

**Pattern 2 — Coin-Stack Pictogram (Isotype/Neurath)**: Each coin = $0.10. Naive = 42 stacked coins, Nunchi = 1.4 coins. Best as OG/share image — thumbnail-legible at 200px. Critical rule: more = more icons, never bigger icons.

**Pattern 3 — Two-Scale Sparkline Strip**: Ghost-baseline trick: translucent rectangle behind Nunchi sparkline shows naive height. 80x20px SVG polyline. Best for dense receipt section — communicates consistency over time.

**Avoid:** log scales, axis breaks, 3D bars.

---

## 7. Competitive Visual Benchmarks

### What Best-in-Class Looks Like (Reference Products)

**Linear** — The benchmark for developer tool design:
- Dark mode as default, not an option
- LCH color space for perceptually uniform themes
- Only 3 core variables per theme: base, accent, contrast
- Content-first: navigation elements deliberately muted so data takes precedence
- 2025 update: warmer gray, less chrome, even more restrained

**Vercel** — The benchmark for developer infrastructure:
- Pure black backgrounds (#000000) — unusually high contrast
- Geist Sans + Geist Mono throughout
- Blueprint grid aesthetic: fine lines, mathematical precision
- Preview URL as the hero interaction
- Moved from "Frontend Cloud" to "AI Cloud" in 2025

**Temporal** — The benchmark for execution infrastructure:
- Three workflow visualization modes: Compact, Timeline, Full History
- Timeline view encodes elapsed time as physical length of connecting lines
- Color language: green=completed, red=failed, dashed=pending, dashed-red=retrying
- The aha moment requires running code — homepage is explanation-heavy (a weakness)

**LangSmith** — The benchmark for agent observability:
- Waterfall/timeline trace visualization
- Tree structure: root run → child runs for each tool/LLM call
- Drill from aggregate → trace → span → raw prompt/response
- Automatic trace clustering for pattern discovery
- Hover-triggered radial gradients on hero imagery (visual polish)

**Devin** — The benchmark for agent demos:
- Three-panel sandbox: terminal, code editor, browser — all controlled by AI
- The SymPy demo: navigating a massive unfamiliar codebase, finding a subtle mathematical error, verifying the fix
- Side-by-side: chat input on left, real-time agent activity on right
- The viral element: SWE-Bench 13.86% vs 1.96% previous SOTA (7x improvement)

### Where Nunchi Must Match or Exceed

| Dimension | Reference | What Nunchi Must Do |
|-----------|-----------|-------------------|
| Typography | Vercel (Geist) | Use Geist Sans + Geist Mono throughout. No serif fonts in the dashboard |
| Color system | Linear (LCH) | Perceptually uniform dark theme with 3 core variables |
| Data visualization | LangSmith (waterfall traces) | Timeline + trace view for agent execution steps |
| Terminal aesthetics | Devin (three-panel) | Clack-style output, Tokyo Night theme, consistent sizing |
| Interaction pattern | Stripe (7 lines) | `nunchi run --share` produces a URL in under 10 seconds |
| Durability demo | Temporal (kill + resume) | Visibly kill, visibly resume, zero work lost |
| Network visualization | None (new category) | Force-directed knowledge graph with live updates |

### Where Nunchi Must NOT Look Like

- **Blockchain explorer** — No hex hashes as primary display. No "block height" as the hero metric. The chain is infrastructure, not the product.
- **Grafana/monitoring tool** — No dense grid of small charts. The dashboard should have 4 views, not 27.
- **LangChain/CrewAI marketing site** — No glossy hero images, no "platform" screenshots with tiny text. Show real data, not mockups.
- **ICO/token launch site** — No countdown timers, no "total value locked" as a hero metric, no gradient-heavy NFT aesthetics.

### Live Benchmark Corner Widget (400x300px)

A persistent corner widget that runs a live A/B comparison during the demo. Four mockup options:

**Mockup 1 — "Bloomberg Two-Tape" (RECOMMENDED PRIMARY)**: Mirrored panels: Roko (cyan #06B6D4) vs LangGraph (orange #F97316). Each panel shows: cost in JetBrains Mono 36pt, pass/fail indicator, progress bar, 60x24 cost sparkline. Footer line: `COST RATIO 31.6x  |  p<0.01  |  ROKO WINS`. Animated: cost ticks via Weave stream every 200ms.

**Mockup 2 — "Esports Gold-Diff"**: LoL-style cost-difference line chart. Big scoreline at top. More visceral, less glanceable than Bloomberg.

**Mockup 3 — "GitHub Actions Pipeline"**: Three rows of 5 circles each. Spinner ring on in-progress steps, checkmarks pop in on completion. Simplest build (2-3hr).

**Mockup 4 — "Optimizely Winner-Declared"**: Confidence interval bar growing rightward. WINNER badge appears at p<0.01 threshold. Best for post-hoc summary rather than live view.

**Implementation pipeline**: agent -> litellm -> Weave logger -> tail JSON -> Node WS bridge -> React widget.

---

## 8. Animation and Interaction Patterns

### Micro-Animations

- **Stat card numbers**: Animate value changes with ease-out over 300ms. Never hard-switch numbers.
- **Progress bars**: Smooth fill with ease-out easing. The bar should feel like it's decelerating as it approaches completion.
- **New data points**: Fade in over 200ms. A brief opacity pulse (0.5 → 1.0) on the data point.
- **Knowledge graph nodes**: New nodes animate in with a scale-up (0 → 1) over 300ms with a subtle bounce.
- **Block feed entries**: Slide in from the top with fade, 200ms.

### Loading States

- **Skeleton loading with shimmer**: Content-shaped placeholders with a left-to-right shimmer during initial load. Reduces perceived load time by 20-30%.
- **Never show a spinner as the only loading indicator.** Always show skeleton shapes that match the content layout.

### Live Indicators

- Green pulsing dot (`animation: pulse 2s infinite`) for "live" status
- "Last updated: 3s ago" timestamp visible in relevant widget corners
- Refresh intervals: 1 second for critical metrics, 10 seconds for trends, 60 seconds for aggregates

### What Not to Animate

- Navigation transitions (they should be instant)
- Data table rows (loading or filtering should be instant)
- The knowledge graph during zoom/pan (should be GPU-smooth, not CSS-animated)

---

## 9. Landing Page Design (nunchi.network)

The landing page is not part of the live demo, but it's the first thing investors see when they visit the URL. It must match the dashboard's design language.

### Hero Section

**Headline**: "The model is the same. The system is the variable."

**Subhead**: Nunchi is the Agent Coordination Plane — the infrastructure layer that separates agent coordination from agent execution.

**Visual**: Animated split-terminal. Left side shows a cost meter ticking upward to $44.86 (red-tinted, slow, painful). Right side shows a cost meter barely moving, stopping at $1.42 (green-tinted, fast). Both run the same task. Numbers are from the Princeton HAL benchmark.

The cost meter animation:
- Loads within 2 seconds of page load
- Runs automatically, no user interaction needed
- Left counter uses `ease-in` (accelerates) — feels heavy
- Right counter uses `ease-out` (decelerates) — feels light
- Total animation duration: 4 seconds
- At completion, a "30x" badge appears with a subtle scale-up

**CTAs**: "Get started" (primary, filled, accent blue) — "Read the docs" (secondary, outlined) — "Watch the demo" (tertiary, text link)

### Seven-Section Scroll

Following the Linear "scroll story" pattern (CSS scroll-snap + Framer Motion viewport animations):

1. **Hero** — Cost meter animation + thesis
2. **Problem** — "41-86% of multi-agent deployments fail" with bar chart
3. **Runtime Loop** — Animated `query → score → route → compose → act → verify → write → react` diagram
4. **Cost Proof** — Side-by-side HAL comparison with interactive breakdown
5. **The Chain** — Network graph visualization, "the thousandth agent joins smarter than the first"
6. **Compliance** — August 2, 2026 date, penalty tier, buyer role created. NOT a countdown timer — plain text with specific date and penalty reads as authoritative
7. **CTA** — Install command, GitHub link, docs link, changelog

### Code Snippet Presentation

Following Stripe's pattern:
- Dark background code block matching page color scheme
- Language tabs (bash, Python, TypeScript)
- Copy-to-clipboard button (appears on hover)
- Shell commands shown with `$` prefix in dimmed color
- Output shown in brighter color below
- 1px border `rgba(255, 255, 255, 0.1)`, border-radius 12px

### Trust Signals

- Real GitHub star count (even if small — authenticity at low numbers reads as honest)
- `/changelog` in the nav with 3-4 shipped items in the last 30 days
- `/customers` with design partner case studies (even if only 1-2)
- `/docs` with protocol specification and SDK reference
- NO fake testimonials, NO mock customer dashboards, NO "trusted by 10,000 developers" without verified numbers

---

## 10. Deck Visual Language

The PDF pitch deck uses different tokens than the landing page and dashboard. The deck is projected in conference rooms (light environments with variable projector quality). The landing page is viewed on developer desktops (dark environment, high-quality displays).

### Deck-Specific Tokens

- **Light backgrounds** — white or near-white. Dark backgrounds project poorly on most conference room projectors.
- **30pt minimum body text**, 60pt+ headers. If it can't be read from 10 feet away, it's too small.
- **15-30 content words per slide**, under 10 on transitions.
- **Total deck: 300-600 words** across all slides (per Alexander Jarvis's research on successful VC decks).
- **12-13 main slides + 8-15 appendix slides** (two-deck practice per Pillar VC, which is what a16z expects).
- **Export as PDF** (never DocSend — friction reads as paranoia). The PDF is the artifact that circulates at the Monday partner meeting.
- **Build in Figma**, present from Keynote (handles offline reliability, presenter mode, and remote control better than browser-based tools).

### Deck Slide Order (R15 locked for Casado meeting)

| Slide | Content |
|-------|---------|
| 1 | Title + thesis: "Nunchi — the durable runtime for production agents" |
| 2 | Why Now (Casado: "I always start with what is the market") |
| 3 | Problem: "Agents broke reliability — again" (Temporal narrative reframe) |
| 4 | Founder (solo founder = team is primary investible asset; show early) |
| 5 | Architecture: control plane / data plane bands (the Casado identity test) |
| 6 | "How it closes the loop": observe → decide → enforce → record |
| 7 | "Let me show you." (blank slide, transition to demo) |
| 8 | Traction: design partner logos + community |
| 9 | Cost comparison: $44.86 → $1.42 (lands HERE with context) |
| 10 | Competition: Power Grid format (not 2x2 quadrant) |
| 11 | Business model + dual-asset structure |
| 12 | Use of funds + milestones tied to Series B |
| 13 | Ask + thesis close (NO dollar amount on slide — per Kirwin's a16z-speedrun essay) |

---

## 11. What "Mind-Blowing" Looks Like Concretely

The goal is not visual spectacle — it's cognitive impact. The investor should leave thinking "I cannot unsee that." Here is what produces that effect:

### The Cost Gap Moment

Two terminals side by side. Same task. The left counter climbs steadily. The right counter barely moves. When both finish, the numbers are $4.18 vs $0.14. The investor sees it happen in real time, not on a slide. The mechanism (cache hits, routing decisions) is visible in the output. This is not a benchmark claim — it is a live demonstration.

### The Kill-and-Resume Moment

The investor expects the demo to be fragile. When you deliberately kill the process and it comes back in 2 seconds, the room shifts. This is Temporal's signature move. It works because it violates expectations. Every other demo carefully avoids failure. This one embraces it.

### The Shareable URL Moment

The `--share` flag produces a URL. The investor opens it on their phone. They see the full execution timeline, the cost breakdown, the ZK proof. The URL is the artifact they forward to their partner. It's the Vercel preview URL for agent runs. Nothing else in the market produces this.

### The "Hand Them the Laptop" Moment

The investor types their own prompt. It works. Fast. The pre-warmed cache covers a range of prompts. This is the Collison move — letting the investor experience the product, not watch it. The experience must be under 10 seconds.

### The Knowledge Compound Moment

The second agent is cheaper than the first. Not because of a better prompt — because it loaded knowledge from the first agent's run. The cost delta is visible. The knowledge line shows "loaded 9 facts from 4 agents." The investor sees the network effect happening in front of them.

---

---

## 12. Current Design System vs Proposed (Implementation Reference)

This section maps the proposed design changes to exact files in the codebase. For the complete codebase reference, see CODEBASE-CONTEXT.md.

### Current ROSEDUST Design Tokens

**File**: `demo/demo-app/src/styles/rosedust.css` (35 lines, 34 CSS custom properties)

The current token system uses a purple-black base with dusty rose accents:
- Backgrounds: `--void: #060608`, `--raised: #0C0A0E`, `--mid: #080810`, `--surface: #100E14`
- Primary accent: `--rose: #AA7088` (dusty pink)
- Text: `--text: #B0A0B0` (mauve-gray)
- Value text: `--bone: #C8B890` (warm off-white)
- Effects: CRT scan-line overlay (in `global.css`), film grain SVG filter, glass morphism borders

### Current Fonts

**File**: `demo/demo-app/src/styles/fonts.css` (2 lines)

Loads from CDN:
- **Instrument Serif** (weight 400) — currently used as display/heading font
- **JetBrains Mono** (weights 400, 600) — currently used for code and data
- **General Sans** (weights 400, 500, 600) — currently used for body text

### Current xterm.js Theme

**File**: `demo/demo-app/src/lib/rosedust-theme.ts` (26 lines)

Maps ROSEDUST colors to ANSI terminal palette. Background: `#0e0c10`, foreground: `#a58e9e` (muted pink), cursor: `#b97894`.

### What Needs to Change

| File | Current | Proposed | Impact |
|------|---------|----------|--------|
| `src/styles/rosedust.css` | ROSEDUST tokens (rose/bone/sage) | Evolved tokens (accent blue, neutral grays, Tokyo Night semantics) | All components re-themed |
| `src/styles/fonts.css` | Instrument Serif + JetBrains Mono + General Sans | Geist Sans + Geist Mono (add `geist` npm package) | Typography across all pages |
| `src/styles/global.css` | CRT scan-line + film grain overlays | Remove overlays OR make them extremely subtle | Atmospheric feel |
| `src/lib/rosedust-theme.ts` | ROSEDUST ANSI mapping | Tokyo Night ANSI mapping | Terminal appearance |
| Every `.css` file (14 total) | Uses `var(--rose)`, `var(--bone)`, etc. | Uses new token names | CSS-variable replacement |

The CSS custom property approach means the token changes propagate automatically — you only need to change the definitions in `rosedust.css`, not every usage. The exception is any hardcoded hex values in component CSS files.

---

*Cross-references: CODEBASE-CONTEXT.md (complete technical reference), DEMO-STRATEGY.md (what and why), DEMO-FLOW.md (beat-by-beat script), DEMO-COMPETITIVE.md (competitive landscape), DEMO-BUILD.md (what to implement).*
