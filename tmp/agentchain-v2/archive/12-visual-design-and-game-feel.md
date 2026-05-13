# Visual Design and Game-Feel

The visual design system for Nunchi's two demo surfaces (CLI terminal and web dashboard), the four primary visualizations (Pulse Globe, Terrain Map for the knowledge graph, Bloomberg Two-Tape benchmark widget, computation receipt), and the game-feel patterns from the v2 prototype that every other section must follow. Every choice is deliberate. Fonts, colors, visualization techniques, and interaction patterns were selected to communicate a specific message to investors and technical evaluators.

---

## 1. Two Demo Surfaces

There are exactly two surfaces that matter:

| Surface | Audience | Purpose | Priority |
|---|---|---|---|
| **CLI terminal** | Investors in the room | "This is real software that works right now" | Primary |
| **Web dashboard** | Technical diligence (CTO, eng lead) | "This scales, and here's the data" | Secondary |

Both must feel like the same product. Same color palette, same typography family, same information density philosophy. A person who sees the CLI demo and later opens the dashboard should have zero cognitive dissonance — they should feel like they are looking at a deeper layer of the same system.

The CLI is what investors see in the pitch meeting. The dashboard is what the technical partner opens during diligence week. Neither can look like a prototype.

---

## 2. Terminal Design

### Font

**Berkeley Mono** ($75 license, variant TX-02) at 24–28pt for demo presentations. Berkeley Mono says "I read the manual; I have taste." It is the font serious infrastructure engineers use. Compare to Geist Mono — fine for web apps, wrong for systems software.

**Fallback: JetBrains Mono** (free, excellent). Used in benchmark widgets and code displays where licensing is a concern.

### Theme

**Tokyo Night** on a `#1A1B26` base background. Full palette:

| Role | Hex | Usage |
|---|---|---|
| Background | `#1A1B26` | Terminal background, all dark surfaces |
| Foreground | `#C0CAF5` | Default text |
| Blue | `#7AA2F7` | Primary accent, links, active states |
| Purple | `#BB9AF7` | Secondary accent, agent identifiers |
| Cyan | `#7DCFFF` | Highlights, success states, Roko brand |
| Red | `#F7768E` | Errors, failures, cost warnings |
| Green | `#9ECE6A` | Pass indicators, completion states |
| Yellow | `#E0AF68` | Warnings, in-progress states |

### Emulator

**Ghostty** (Mitchell Hashimoto's GPU-rendered terminal). Zero frame drops during demos. A janky terminal during a live demo destroys credibility instantly. Ghostty renders via Metal/Vulkan, so scroll, resize, and high-throughput output all stay at native refresh rate.

### Symbols

Clack-style box-drawing and indicator characters: `◆ ◇ │ └ ✔ ✖ ⚠ ℹ ❯ → dots spinner`. **No emoji. Ever.** Emoji in CLI output signals "weekend project," not "infrastructure." The Clack symbols (from the Clack CLI framework by natemoo-re) communicate structure without being cute.

### Color accent constraint

Two colors are off-limits as primary accents:

- **Green:** Supabase owns green in the developer tool space. Using it creates unconscious association with "hosted Postgres."
- **Orange:** Replit and Hacker News own orange. Creates association with "consumer coding tool" or "forum."

Nunchi's primary accent is **cyan/blue** (`#7DCFFF` / `#7AA2F7`). Sits in the same family as Linear and Vercel's accent choices, signaling "premium developer infrastructure."

---

## 3. Dashboard Design

### Design evolution

The dashboard started as **ROSEDUST** — a dark purple-black base with dusty rose accents. Atmospheric and distinctive but had problems: the rose accent was too warm for data-heavy views, contrast was insufficient for dense tables, and it read as "aesthetic project" rather than "production infrastructure."

The evolution:

| Attribute | ROSEDUST (original) | Current direction |
|---|---|---|
| Background | Dark purple-black | Near-black (`#0A0A0F` to `#111118`) |
| Accent | Dusty rose | Blue `#4A9EFF` (or `#0070F3` Vercel blue per R15 lock) |
| Display font | Custom serif | **Geist Sans** (matches Vercel/Linear tier) |
| Monospace | Berkeley Mono | Berkeley Mono (kept) |
| Spacing | Generous, atmospheric | Tighter, higher information density |
| Contrast | Low (mood over readability) | Higher (data must be scannable) |
| Chrome | Minimal | Minimal (kept — this was right) |

### What to keep from ROSEDUST

- Dark backgrounds everywhere. No light mode. Light mode is a distraction for v1.
- Monospace-heavy layouts. Tables, logs, and metrics all render in mono.
- Minimal chrome. No gradients, no drop shadows, no rounded-corner cards with padding. Content sits directly on the dark surface with thin dividers.
- Atmospheric depth. Subtle background texture or noise that prevents the "flat void" feeling. A 1–2% noise overlay on the base color.

### What changed

- Accent shifted to `#4A9EFF` / `#0070F3` — cooler, higher contrast, reads as "data" not "mood."
- Display font is Geist Sans. The font Vercel uses. Signals membership in the tier of developer tools that take typography seriously.
- Spacing tightened. ROSEDUST had generous whitespace that looked beautiful on a single screen but wasted space when showing 50 agents or 200 knowledge entries.
- Contrast increased across the board. WCAG AA minimum for all text on dark backgrounds.

### Four demo views

The dashboard has exactly four views for demo purposes. Everything else accessible behind navigation but not in the demo flow.

1. **Cost Dashboard** — Real-time and historical cost data. Per-agent spend, cumulative totals, cost-per-task breakdowns, the 30x comparison visualization.
2. **Agent Fleet** — All running agents with status, current task, resource consumption, and communication graph. Think "kubectl get pods" but for AI agents.
3. **Knowledge Graph** — The terrain/mycelial visualization (see section 4). Shows the system's accumulated knowledge, confidence levels, and decay patterns.
4. **Chain View** — On-chain settlement activity. Transaction hashes, verification status, cost receipts. This view exists even before the chain is live — it shows simulated data from `mirage-rs` to demonstrate the architecture.

A "Demo Mode" toggle in the top bar should auto-cycle through these four views on a 45-second timer for investor meetings where the presenter wants the screen to tell the story while they talk.

---

## 4. Knowledge Graph Visualization

The knowledge graph needs a visualization that communicates two core concepts: **compounding** (knowledge gets more confident over time) and **demurrage** (unused knowledge decays). Four approaches in priority order.

### Primary: Terrain Map

**Technology:** d3-contour + Canvas 2D.

The core insight: a topographic map is the perfect metaphor for knowledge with decay.

- **Compounding = elevation.** As confidence in a knowledge entry accumulates (more citations, more successful uses), its peak rises. High peaks are things the system is very confident about.
- **Demurrage = erosion.** Unused knowledge decays. Peaks shrink. Valleys widen. The terrain literally erodes over time, just like real knowledge that isn't reinforced.

**Implementation details:**

- d3-contour ships `contourDensity().weight()` out of the box. The weight function maps to confidence scores from the neuro store.
- Position knowledge entries on the 2D plane via UMAP or t-SNE dimensionality reduction on the HDC fingerprint vectors.
- Per-frame: run KDE (kernel density estimation) on a 128×128 grid with 12 contour thresholds. On an M1 MacBook Pro this takes 3–6ms per frame, well within the 16ms budget for 60fps.
- Total implementation delta: approximately 200 lines of code on top of existing d3 setup.
- Color ramp: low elevation = dark blue/black (valley), high elevation = cyan/white (peak). Matches the Tokyo Night palette naturally.

**Implementation core (Path A):**

```js
const N = 128;
const grid = new Float32Array(N*N);
// each tick: zero grid, splat each node's exp(-lambda*dt) weight as 2D Gaussian
const contours = d3.contours().size([N,N]).thresholds(12);
const polys = contours(grid);
const color = d3.scaleSequential([0, max], d3.interpolateViridis);
ctx.clearRect(0,0,w,h);
for (const p of polys) { ctx.fillStyle = color(p.value); ctx.beginPath(); path(p); ctx.fill(); }
```

Position via UMAP/t-SNE projection of node embeddings (deterministic — not d3-force-relaxed). Elevation/contour color = aggregated confidence weighted by inverse demurrage.

**Verified reference URLs:**
- d3-contour module: https://d3js.org/d3-contour and https://d3js.org/d3-contour/density (the `density.weight()` accessor)
- Volcano contours: https://observablehq.com/@d3/volcano-contours/2
- Density contours: https://observablehq.com/@d3/density-contours
- Cloud Contours (animated field): https://observablehq.com/@d3/cloud-contours
- Frymire walkthrough: https://medium.com/two-n/an-alternative-to-choropleth-contour-density-maps-in-d3-js-93e1fdbdc4e

### Secondary: Mycelial / Physarum Simulation

**Use case:** landing page hero animation. Not for the data dashboard.

Physarum polycephalum (slime mold) simulation. Agents deposit chemical trails, paths that get reinforced grow stronger, paths that don't fade away. This is literally the same algorithm NASA's Jet Propulsion Laboratory used to map the cosmic web (dark matter filament structure).

- 200,000 simulated agents running at 60fps on M1.
- Paths reinforce themselves — exactly the metaphor for how Roko's knowledge compounds.
- Visually stunning and scientifically grounded. Not just eye candy.

References: Sage Jenson physarum (https://sagejenson.com/physarum), fogleman/physarum (Go reference, JS port straightforward — https://github.com/fogleman/physarum), Polyphorm cosmic-web tool (https://github.com/CreativeCodingLab/Polyphorm) — same algorithm NASA used to map dark matter.

### Tertiary: Star Map

**Technology:** d3-celestial.

Knowledge entries as stars. Magnitude (brightness) maps to decay — bright stars are fresh, confident knowledge. Dim stars are decaying. Dead stars are archived. Less computationally impressive than terrain or physarum but creates an immediately recognizable metaphor. Good fallback if other approaches have performance issues.

### Fallback: Force-Directed Graph

**Technology:** d3-force + Canvas 2D.

Traditional knowledge graph visualization. Nodes are knowledge entries, edges are relationships, force simulation keeps it dynamic. Works and is well-understood, but it's also what every knowledge graph demo looks like. Doesn't visually distinguish Nunchi from competitors. Use only if other approaches aren't feasible within the timeline.

### Comparison

| Concept | Stack | New deps | LOC delta | GPU? | Demo-glance verdict |
|---|---|---|---|---|---|
| **Terrain (Path A — d3-contour + Canvas 2D)** | existing d3 stack | `d3-contour` (transitive) | ~200 | No | **Strong** |
| Terrain (Path B — three.js heightmap shader) | three.js | `three` | ~600 | Yes | Cinematic upgrade |
| Cellular automaton / reaction-diffusion | gpu-io/regl + Pixi | `gpu-io` or `regl` | ~500 | Mandatory | Too noisy |
| Star map / constellations | Canvas 2D | `d3-celestial` (BSD-3) | ~300 | No | Slightly generic |
| Mycelial / physarum network | regl ping-pong | `regl` | ~600 | Mandatory | Needs metaphor explanation |
| River delta (Sankey + particles) | Canvas 2D | `d3-sankey` | ~350 | No | Flow yes, graph no |

**Avoid as primary:** CA (too twitchy for 30s read) and River Delta (flattens graph topology, fights the "graph" beat).

---

## 5. The Pulse Globe (Cold-Open Hero)

The pitch opens with a globe. Agents fire across it. The audience watches real coordination happen before anyone speaks. This is the first thing investors see.

### Technology

- **three-globe** library by Vasturiano (MIT license). Purpose-built for globe visualizations on Three.js.
- **Three.js** for the 3D scene.
- **UnrealBloomPass** (Three.js post-processing) for the glow effect on arcs.

**Skeleton:** clone **janarosmonaliev/github-globe** (MIT, https://github.com/janarosmonaliev/github-globe — the canonical community implementation). Day-1 buildable.

**Why three-globe over alternatives.** GitHub's own globe (https://github.blog/2020-12-21-how-we-built-the-github-globe/) and Stripe's (https://stripe.com/blog/globe) are proprietary (technique-only, not source). Cobe is built for the dotted aesthetic and tops out at ~100 arcs. deck.gl's GlobeView is experimental with no rotation/pitch. three-globe (vasturiano, MIT, https://github.com/vasturiano/three-globe) is the API sweet spot: declarative `arcsData()` / `pointsData()` / `ringsData()` / `particlesData()`, automatic BufferGeometry merging.

### Agent Lens colors

Each agent type has a distinct color, called a "Lens":

| Agent Type | Color | Hex |
|---|---|---|
| Planner | Purple | `#7C5CFF` |
| Executor | Cyan | `#22D3EE` |
| Retriever | Emerald | `#34D399` |
| Critic | Pink | `#F472B6` |
| Human | Amber | `#FBBF24` |
| System | Slate | `#94A3B8` |

### Performance

- **5,000 arcs simultaneously at 60fps on an M1 MacBook Pro.**
- Arc dash animation duration: 1500ms (fast enough to feel alive, slow enough to track).
- Atmosphere color: `#7C5CFF` (planner purple — the "brain" of the system dominates the visual).

### Exact parameters

- Particle size 2.5–4.0px screen, `sizeAttenuation:false`
- Trail: `arcDashLength=0.4`, `arcDashGap=0.6`, `arcDashAnimateTime=1500ms`
- Altitude: `arcAltitudeAutoScale=0.5`, capped at `arcAltitude=0.45`
- Origin ring: `ringMaxRadius=4deg`, `ringPropagationSpeed=5deg/s`, `ringRepeatPeriod=800ms`, color `t => rgba(<lensRGB>, sqrt(1-t))`
- Globe: radius 100, `MeshPhongMaterial({color:0x0a0e1f, emissive:0x0a0e1f, shininess:0})`, `atmosphereColor('#7C5CFF')`, `atmosphereAltitude(0.18)`
- Scene: `Fog(0x05060a, 200, 800)` + `UnrealBloomPass(strength=0.9, radius=0.6, threshold=0)` for the visceral glow
- Slider binds to `emissionsPerSecond in [0,200]`, default 30; also drives BusBridge throughput so it's a real load knob

### Avoiding particle mush at 5,000

- Single BufferGeometry/draw call for `THREE.Points`
- `AdditiveBlending + depthWrite:false`
- Soft 32x32 RGBA radial-falloff sprite (`pow(1-r, 2)`)
- Per-Lens cap of ~1,000 simultaneous arcs in ring buffer
- Back-face cull via three-globe's planet occlusion + custom `discard` shader (`normal*view < -0.1`)
- LOD drops trail dash count 8 → 3 when `cameraDist > 400`

**Frame-budget reality:** GPU does all 5k in parallel in one draw call (~0.3ms on M1); CPU is one buffer write per frame (~0.5ms). Plenty of headroom.

### The slider

The globe has a slider control. **This is not decorative.** The slider drives two things simultaneously:

1. **Visual:** the emission rate of arcs on the globe. Slide right, more arcs fire.
2. **Real:** the actual BusBridge throughput in the running Roko instance. The slider is wired to real infrastructure, not just animation parameters.

This is the demo's key moment. The investor realizes the visualization isn't a mockup — it's a live view of a real system. The slider controls the actual system.

### Hero animation spec for landing page

Full-bleed mount, headline `mix-blend-mode: screen`, auto-rotate 0.3deg/s, pause on `mouseenter`, viewport-tiered emission (desktop 30 / tablet 15 / mobile 8), SSR with SVG screenshot crossfade-to-canvas via Web Animations API (the GitHub-globe trick that prevents first-frame jank), DPR `Math.min(2, devicePixelRatio)`. Asset budget < 400kB gz total.

### Why the Pulse Globe matters

The Pulse Globe is the literal visualization of Aubakirova's Big Ideas 2026 line: *"It's not architected for a single agentic 'goal' to trigger a recursive fan-out of 5,000 sub-tasks, database queries, and internal API calls in under milliseconds."* The 5,000 sub-task fan-out IS the Pulse Globe spec. Open the Casado pitch on the Pulse Globe with this quote flashed on the slide. She wrote those exact words five months before the meeting.

---

## 6. Computation Receipt — Hybrid Mockup D (Recommended)

After an agent run completes, users get a URL they can share: `nunchi.dev/runs/{id}`. This page shows the full execution trace and produces a downloadable computation receipt.

### Execution trace

The trace visualization is a **multi-track, time-aligned, canvas-rendered timeline** — convergence of Chrome DevTools Performance tab and Datadog APM traces:

- Multiple horizontal tracks, one per agent.
- Time flows left to right.
- Blocks represent agent actions, colored by agent type.
- Hover over any block for details; all tracks sync to the same time cursor.

**Implementation details:**

| Element to steal | Source | Implementation |
|---|---|---|
| Multiple synchronized tracks sharing one x-axis | Chrome DevTools Performance | Canvas2D + React overlay layer; tracks for agent execution flame, $/sec ribbon, token throughput, latency p99, agent-state "screenshots" |
| Per-agent categorical color palette (hash-to-HSL) | Jaeger UI | One hue per agent; the eye instantly reads system shape |
| Mini-map context strip at top | Jaeger | CSS `transform` for spans (not SVG `<rect>`) for perf |
| Drag-select → outlier histogram | Honeycomb BubbleUp | D3 + canvas heatmap, KL-divergence ranking on dimensions |
| Shade-by-cost ramp (lightness encodes $) | Datadog Trace View 2023 | hue=agent, lightness=cost; expensive spans literally glow |
| Log/event ↔ timeline hover sync | Datadog | Vertical cursor on flame graph as user hovers cost line items |
| Animated dashed "marching ants" for in-flight events | Temporal | CSS `@keyframes` on `stroke-dasharray`; the single best "this is live" signal |
| vis-timeline as base library | Temporal | https://github.com/visjs/vis-timeline — drop-in, handles thousands of events |
| WebGL flame rendering for huge traces | Speedscope | Future-proof; not v1 |
| Differential flame graph (red=slow vs blue=fast) | Brendan Gregg | Naive baseline vs Nunchi-coordinated rendered side-by-side |
| Inline $ cost labels on every bar (mono right-edge) | LangSmith — minimum viable copy | Most agent traces hide this; Nunchi must show it |
| Power-user command palette (`Ctrl+Shift+P`) | Perfetto | Optional; doesn't clutter casual viewers |

### Computation receipt — four mockups

Four hand-off-ready visual languages for the ZK proof artifact:

#### Mockup A — "Notarized Page" (paper-cream + ink-black + wax-seal)

720px-wide A4-portrait feel.
- Headings: EB Garamond / Cormorant Garamond 28pt small-caps
- Body: Source Serif 4 15/24
- Hashes: JetBrains Mono 13
- Palette: paper-cream `#FAF6EE`, ink `#1A1A1A`, hairline `#C9C2B2`, wax-red `#7E1F1F`, apostille-gold `#A8895C`
- Ornament: SVG-generated guilloche border (`<pattern>` + `<feTurbulence>`), centered wax-seal SVG, apostille-style certificate number
- Feel: Sotheby's lot certificate

#### Mockup B — "Etherscan Receipt" (dark + neon, for ZK-native viewers)

- Background: `#0B0E14` / panel `#11151D` / text `#E6EAF2`
- Accent magenta `#FF61D2` for proof-only callouts; success-green `#3FB950`, link-cyan `#58A6FF`
- Typography: Inter for labels, IBM Plex Mono for hashes/numbers
- Layout: top status pill + property/value table (Run ID, From-Agent, To-Agent, Spans, Cost, Wall Time, Tokens In/Out, Verifier Contract, Proof Size, Proving Time) + "Logs / Internal Calls" expander
- Pulsing rotating-cube icon next to proof hash (CSS `@keyframes` 2.4s opacity 0.6→1)

#### Mockup C — "Lab Notebook" (lined paper + signed/dated)

Two-page spread.
- Headings: Caveat / Kalam for "Witnessed by" only
- Body: iA Writer Quattro / Lora 15/26
- Palette: notebook-cream `#F5EFE0`, line-blue `#A8B5C8`, ink-blue `#1B2A4E`, red-pen accent `#B23A3A`, soft gold `#C8A961`
- Includes deterministic SVG identicon (Github-identicon-style) generated from run hash
- "[VERIFIED — NUNCHI]" red rubber-stamp graphic at 7 degree rotation
- Feel: Newton's notebook; admissible in IP litigation

#### Mockup D — Hybrid (RECOMMENDED DEFAULT)

Dark canvas (Mockup B aesthetic) for execution timeline + cost; embedded paper-cream Mockup-A "receipt card" floats lower-right at ~340x480px with subtle drop-shadow.

- Click flips card (3D CSS `rotateY(180deg)`) to show technical proof internals
- The card is a downloadable PDF (server-side `puppeteer` render) for invoice attachment
- **Why this wins:** the contrast between active dark canvas and still cream card visually encodes the pitch — *we run agents at scale and produce a permanent artifact*. The card is the literal "thing that leaves the room."

References fetched: Etherscan tx-page docs, Blockscout walkthrough, Certifier.io COA design tips, RISC Zero zkVM "receipt" terminology (arxiv:2502.07063).

### Cost comparison — three patterns for $0.14 vs $4.18 (~30x)

#### Pattern 1 — "Crushed Bar" (RECOMMENDED HERO)

Two horizontal bars stacked. Top: `$4.18 — Naive baseline` at 100% width, hot-red `#E5484D` with subtle scan-line texture. Bottom: `$0.14 — Nunchi` at 3.3% width, muted-green `#3FB950`. Dotted vertical at 3.3% mark labeled `30x less` in mono. Pure HTML/CSS, two divs, no library. On-viewport-enter animation: green bar draws 0 → 3.3% over 800ms with `cubic-bezier(0.2, 0.8, 0.2, 1)`.

**Why it wins for 30x:** linear bars are at their sweet spot at this ratio — at 100x the small bar becomes invisible; at 10x the asymmetry is unimpressive. Tufte-correct (proportional ink, no axis break, no log trickery).

#### Pattern 2 — Coin-Stack Pictogram (Isotype/Neurath)

Each coin = $0.10. Naive column = 42 stacked coins. Nunchi column = 1.4 coins. Single caption: *"Each circle = $0.10 of LLM spend."* React/SVG loop or Flourish pictogram template. Critical rule (Neurath's first law): more = more icons, never bigger icons. Best as the OG/share image — thumbnail-legible at 200px wide.

#### Pattern 3 — Two-Scale Sparkline Strip

`Naive $4.18 (tall sparkline) → Nunchi $0.14 (flat sparkline)`. Wilke's "ghost-baseline" trick: translucent rectangle behind Nunchi sparkline shows the height the naive line would have at this scale; eye instantly sees empty space. Inline SVG `<polyline>`, 80x20px. Best for the dense receipt section — communicates consistency, not just average savings.

**Design rules:** Avoid log scales (academic; undersells the gap), axis breaks (loses honesty), 3D bars or stylized icons that distort area.

**Mapping:** Pattern 1 → landing page hero. Pattern 2 → OG/share card. Pattern 3 → receipt section in shareable URL page.

---

## 7. Live Benchmark Widget — Bloomberg Two-Tape

A 400×300px overlay in the corner of the dashboard that shows live benchmark results.

### Bloomberg Two-Tape (RECOMMENDED)

Two mirrored panels side by side:

| Left Panel (Roko) | Right Panel (LangGraph) |
|---|---|
| Cyan accent (`#7DCFFF`) | Orange accent |
| Cost in JetBrains Mono 36pt | Cost in JetBrains Mono 36pt |
| Pass/fail indicators per task | Pass/fail indicators per task |
| Sparkline (cost over time) | Sparkline (cost over time) |

Footer: cost ratio (e.g., "30.2x") and p-value (statistical significance).

This design was chosen because it is the most glanceable. An investor can understand the competitive position in under 2 seconds. The Bloomberg aesthetic signals "quantitative rigor" — these are real numbers, not marketing claims.

**Animated elements:** cost ticks up via Weave stream every 200ms; sparkline rolling 30s window; pass icons populate left-to-right; ratio recomputes per-task. Static elements: layout, headers.

**Implementation pipeline:**

```
agent → litellm callback → Weave logger → tail JSON → Node WebSocket bridge (FastAPI ~40 lines) → React widget @ ws://localhost:8765/stream
```

Statistical significance via sequential proportion z-test: `proportion_z_test(roko_pass, n, lg_pass, n)`; declare WINNER at p<0.01.

### Alternatives considered

- **Esports Gold-Diff** (Inspired by League of Legends): single line chart where y-axis represents cost difference. More visceral and dramatic; less glanceable.
- **GitHub Actions Pipeline:** three rows (tasks) by 5 circles (steps). Simplest to build, least visually impressive.
- **Optimizely Winner-Declared:** confidence interval visualization. Best for post-hoc summary, not live display.

---

## 8. Five Game-Feel Patterns (v2 Prototype Lessons)

The v2 prototype builds three full game-feel miniatures and establishes five patterns every other section must follow.

### Pattern 1: Every section has a primary visual object that IS the mechanism

The visual doesn't illustrate the concept — it executes it. The hero's 3D stack isn't a diagram of Roko; it's Roko. The freeway isn't a chart; it's cognitive gating happening. The swarm isn't a graph of a network; it's the network. Text becomes annotation around the object, never the main vehicle.

### Pattern 2: Every mechanism has a live state counter somewhere in the HUD

The freeway shows "THROUGHPUT · N ticks" and "SAVED VS ALWAYS-FRONTIER · $N.NN" — counters that accumulate as you watch. The swarm shows InsightStore size, gate-pass %, cost/episode. These numbers tick in real time. They're how you know you're watching something alive, not a rendered animation.

### Pattern 3: Every scene has one legible interaction — not many

Hero: drag to orbit, hover for layer info. Freeway: one slider (prediction error). Swarm: one slider (agent count). **Do not cram 4 controls into one scene.** One axis of control per miniature. The VC plays one thing, sees it do something, moves on.

### Pattern 4: Three.js when the spatial/depth dimension teaches something; 2D canvas when not

Hero (spatial stack), Swarm (3D graph of nodes in space) — Three.js earns it. Loop (orbital diagram), Freeway (top-down flow), VCG (bar chart pit) — 2D canvas is cleaner, renders faster, reads better. **Don't force 3D where 2D tells the story better.** The rule: if rotating the camera would reveal new information, use 3D. If not, use 2D.

### Pattern 5: Animated phosphor fade instead of full clear on 2D canvases

All 2D canvases paint over with `rgba(6, 6, 8, 0.18-0.22)` each frame rather than `clearRect`. This creates trails — the CRT phosphor afterimage. Any moving element leaves a rose-colored ghost that fades over ~5–10 frames. **This is the single effect that makes 2D canvas work in ROSEDUST; don't skip it.**

### Color and pulse rules specific to 3D scenes

- **Materials:** `MeshPhongMaterial` with `emissive` set to the ROSEDUST token, `emissiveIntensity` in 0.15–0.6, `transparent: true, opacity: 0.65–0.9`. Glass-like, never plastic.
- **Edge highlights:** always add `EdgesGeometry` + `LineBasicMaterial` on top of box/mesh geometry. The edge lines are where rose gets to sing.
- **Lighting:** one warm rose directional key light, one cool indigo fill, one rose point-light as rim. **Never white light.**
- **Fog:** `FogExp2(0x060608, 0.02–0.035)` on every scene. Depth disappears into the void. Always.
- **Particle additive blending:** every particle system uses `AdditiveBlending` + `depthWrite: false` + `transparent: true`. This is what gives the phosphor glow.

---

## 9. Specific Game-Feel Miniatures (v2 → v1 Production)

Five additional miniatures specified for the production landing page beyond the v2 prototype.

### The Lattice (Section 6 / Chain Visualization)

3D chain tree rendered as **glass prisms**. Each prism is a block. Blocks hang from a root at the top and branch downward (Kauri BFT dissemination shape).

- **Geometry:** each block is a translucent `BoxGeometry(0.8, 0.8, 0.8)` with `MeshPhysicalMaterial` — `transmission: 0.8, thickness: 0.5, roughness: 0.1, ior: 1.5` — actual glass. Emissive indigo at low intensity; gets brighter as insights accumulate inside.
- **Arrangement:** ternary tree, 4 levels. Level 0 = root, level 3 = leaves. 1 + 3 + 9 + 27 = 40 blocks total.
- **Connections:** thin rose-deep lines between parent/child blocks. Lines have a traveling "block-pulse" sprite — a small bright rose dot traveling root → leaves every 400ms (the Nunchi blockchain block time).
- **Insights inside blocks:** each block contains 3–8 smaller glowing motes. Motes drift slowly inside the glass cube like fireflies in a jar. When a new insight is "deposited," a new mote appears in a random leaf block with a pop animation.

**HUD elements (absolute-positioned over the canvas):**

```
TOP LEFT:    BLOCK                   #0x0011a7e
             ────                    ~50ms blocks · BFT finality

TOP RIGHT:   ISFR                    3.72% ▲
             14-DAY MEDIAN           +0.03 since last tick

BOT LEFT:    INSIGHTSTORE            N entries
             6 TYPES · DECAYING      → next prune in 4m 12s

BOT RIGHT:   TPS                     12,847
                                     simplex bft · O(n) messages
```

The block number increments every ~50–400ms. The ISFR rate jitters slightly with each block (mean-reverting random walk around 3.72%, ±0.05 range).

**Performance.** 40 glass blocks with `MeshPhysicalMaterial` and transmission is expensive. If it stutters, drop to `MeshStandardMaterial` with `transparent: true, opacity: 0.4` — 95% of the look, 20% of the cost.

### Yield Perps Gap (below the lattice)

Single arresting SVG. 1000×140 viewport:

```
TRADFI IR DERIVATIVES  ████████████████████████████████████████████  $668T
                       └────────────────────────────────────────────┘
                                                                     log(notional)

ON-CHAIN IR PRODUCTS   ▏                                              $100M
                       └┘
```

Text overlay: *"6 orders of magnitude — Nunchi 1st app"* in bone, centered.

Animate bars drawing left-to-right on scroll-into-view. Let the gap speak.

### HDC Playground (Section 5 / Stigmergic Collective)

The interactive that wins Twitter. Two side-by-side 64×32 grids. Each grid represents a compressed view of a 10,240-bit HDC vector (showing 2,048 bits). Lit cells = 1, dark cells = 0.

Above each grid: a text input. The input's text is hashed deterministically (FNV-1a) into the 2,048-bit pattern. Same text → same pattern, always.

Below the two grids, a third region: the **interference pattern** — the XOR of the two grids, visualized. Cells where both grids agree are dim (rose-deep). Cells where they disagree are bright (rose-bright). The density of bright cells = Hamming distance.

**Interaction.** As you type, both grids update live (debounced 100ms). The XOR interference grid recomputes. Readout below shows:

```
HAMMING DISTANCE        847 / 10,240    (8.3%)
SIMILARITY              0.917
CROSS-DOMAIN TRANSFER   ✓ TRIGGERED
```

When similarity > 0.85, a badge fires with a rose-bright glow pulse and the following sentence types out below (character-by-character at ~30ms/char):

> *"These encode as structurally equivalent patterns. Knowledge from the trading agent is now available to the coding agent's context window."*

**Preset pairs (VC demo mode):**

- `verify before commit` ⇔ `verify before position` → high similarity (structural equivalence)
- `optimize compile` ⇔ `oat milk latte` → very low similarity (control)
- `reduce retry backoff` ⇔ `lower position size after drawdown` → medium-high (noisy-but-real)
- `bind(high-surprise, escalate-model)` ⇔ `bind(high-vol, reduce-leverage)` → very high (HDC bind op)

**Why this matters for the VC.** HDC is the weirdest, most-provable-on-demand piece of the tech stack. A VC who's seen a thousand AI pitches has never seen two text inputs interfering with each other as bit-fields. This is the visual that goes on their partner meeting slide.

### Telemetry Triptych (Section 7 / End-to-End Demo)

The kill shot. Three panels side-by-side, 45%/30%/25% width split:

- **LEFT (45%) — the terminal.** xterm.js (or asciinema player) playing the scripted coding-agent run. ROSEDUST themed. 80 columns wide.
- **CENTER (30%) — the loop monitor.** Live-updating canvas showing the 6-phase orbit, in miniature. As the terminal reports which phase it's in, the corresponding node in the orbit lights up. Current tier (T0/T1/T2) shown as a badge. Cost accumulates as a number below the orbit.
- **RIGHT (25%) — the chain monitor.** Simplified version of the Lattice. When the terminal emits an `insight.insert(...)` line, a new glowing mote spawns in a leaf block with a pop animation. Running counter shows total insights deposited this session.

**All three panels update together.** The terminal is the driver; the other two are slaves.

### The "play again" button

After the run completes, a "RUN IT AGAIN — WATCH IT START SMARTER" button appears. On click, the terminal replays, but the scripted transcript is slightly different:

- OBSERVE shows `memory_hit: 0.95` instead of 0.78 (higher — it remembers from last run)
- GATE routes to T0 on the first tick (cheap-lane — it recognized the task)
- No gate failure this time — the insight from the first run prevented the test-gate miss
- Total cost: $0.008 instead of $0.043 (5× cheaper)
- Chain panel shows the insight promoting from STAGED to PERMANENT

**This is the kill shot.** Session N+1 is categorically better. The VC watches it happen.

---

## 10. Competitive Visual Benchmarks

These are the products Nunchi's design must hold up against. Each represents a standard in its category.

| Product | What to learn |
|---|---|
| **Linear** (developer tool design benchmark) | Information density. Linear packs enormous amounts of data into clean layouts. Dark mode as default, LCH color space, content-first. |
| **Vercel** (infrastructure design benchmark) | Typography hierarchy. Vercel's pages are readable at a glance because the type scale is perfect. Pure black backgrounds (`#000`), Geist font family. |
| **Temporal** (execution visualization benchmark) | Three visualization modes for workflow execution. Temporal made execution visualization the product. The viz IS the value prop. Nunchi should do the same for agent coordination. |
| **LangSmith** (observability benchmark) | Trace depth. Drill from "this run cost $4" to "this specific LLM call in this specific chain cost $0.12" in two clicks. |
| **Devin** (agent demo benchmark) | Liveness. Devin's demos feel alive because you can see the agent thinking. Roko's CLI output needs to feel equally alive. |

### Where Nunchi must NOT look like

- **Blockchain explorer (Etherscan aesthetic):** tables of hashes signal "crypto project" not "developer infrastructure."
- **Grafana:** enterprise monitoring dashboards signal "ops tool" not "product."
- **LangChain marketing site:** gradient-heavy, illustration-heavy. Signals "we raised money and hired a design agency" not "we built something real."
- **ICO site:** token-forward, countdown timers, "join the revolution" copy. Nunchi has a token but it's infrastructure, not speculation.

---

## 11. Performance Budget (All Scenes Combined)

Production targets — tight but achievable with the optimizations noted:

| Scene | Target triangle budget | Max instances | Fallback trigger |
|---|---|---|---|
| Hero stack | 3K triangles, 200 particles | N/A | If FPS < 40 on 2021 M1 MacBook Air with CPU 4× throttle, drop knowledge particles |
| Loop orbital | 2D canvas (no triangles) | N/A | If FPS < 40, drop flow particles, keep nodes static |
| Freeway | 2D canvas (no triangles) | 40 max vehicles on screen | If FPS < 40, cap at 20 vehicles |
| VCG | 2D canvas (no triangles) | N/A | Already cheap |
| Swarm | 1K agents × ~80 triangles = 80K, + 40 arcs × 23 segs | 1000 instances | If FPS < 30, cap agent count at 500 and warn in HUD |
| Lattice (chain) | 40 glass blocks × ~100 triangles + 120 motes = 4K | N/A | Swap MeshPhysicalMaterial for MeshStandardMaterial |
| HDC playground | SVG — trivial | N/A | Never a problem |
| Demo triptych | xterm + 2 small canvases | N/A | Already cheap |

**Only one heavy Three.js scene renders at a time.** Use IntersectionObserver to pause `requestAnimationFrame` on scenes not in view. Critical — without it, 4 concurrent Three.js renderers will thrash.

```js
let rafId = null;
let inView = false;
const io = new IntersectionObserver(entries => {
  inView = entries[0].isIntersecting;
  if (inView && !rafId) animate();
}, { threshold: 0.1 });
io.observe(sectionEl);

function animate() {
  if (!inView) { rafId = null; return; }
  // ... render ...
  rafId = requestAnimationFrame(animate);
}
```

**Final pass.** Scroll the full page on a 2021 M1 MacBook Air with Chrome CPU throttled 4×. The whole experience at 60fps is the quality bar. 55fps is acceptable if sustained. Below 45fps, reduce quality.

---

## 12. Accessibility

The game-feel visuals raise accessibility concerns:

- **Every section must have a "still-image" fallback.** If `prefers-reduced-motion: reduce`, each scene renders one frame and freezes. Still beautiful. Not animated.
- **Keyboard controls.** The slider for PE (Gate) and agent count (Swarm) must be keyboard operable (arrow keys). The orbital loop canvas should have arrow keys for next/prev phase.
- **Live regions.** HUD counters (ticks, saved, etc.) update frequently; they should NOT be wrapped in `aria-live` regions because that would spam screen readers. Instead, provide a single summary button ("show current state") that reveals a static snapshot.
- **High-contrast mode.** If the user has a high-contrast OS setting, bump rose_bright and bone opacity to 100%, remove scanlines, increase font weight. Do not change the color scheme — ROSEDUST remains — but push the signal.
- **Color contrast.** Rose (`#aa7088`) on bg_void (`#060608`) measures ~5.8:1 — meets AA for large text. Body copy (`#988090` on `#060608`) measures ~7.2:1 — AAA. Ghost/phantom colors are intentionally below contrast thresholds — they are atmospheric and non-essential. Critical information must be in `text_primary` or brighter.

---

## 13. Visual Design Summary

| Element | Content |
|---|---|
| Two demo surfaces | CLI terminal (primary) + web dashboard (secondary) |
| Terminal font | Berkeley Mono ($75) at 24–28pt; JetBrains Mono fallback |
| Terminal theme | Tokyo Night on `#1A1B26` |
| Terminal emulator | Ghostty (GPU-rendered, zero frame drops) |
| Symbols | Clack-style — `◆ ◇ │ └ ✔ ✖ ⚠ ℹ ❯ → dots spinner`. NO emoji ever. |
| Off-limits accents | Green (Supabase), Orange (Replit/HN). Use cyan/blue (`#7DCFFF` / `#7AA2F7`) |
| Dashboard accent (R15) | `#0070F3` Vercel blue |
| Dashboard font | Geist Sans display, Berkeley Mono code |
| Four demo views | Cost Dashboard, Agent Fleet, Knowledge Graph, Chain View |
| Knowledge graph | Terrain Map (d3-contour + Canvas 2D), ~200 LOC delta. Mycelial / Physarum for landing hero. |
| Pulse Globe | three-globe + Three.js + UnrealBloomPass. 5,000 arcs at 60fps M1. Slider drives real BusBridge throughput. |
| Computation receipt | Hybrid Mockup D — dark canvas + cream paper card, downloadable PDF |
| Cost comparison hero | Crushed Bar (two horizontal bars, 100% vs 3.3%, animated) |
| Live benchmark widget | Bloomberg Two-Tape (Roko cyan / LangGraph orange, 400×300px) |
| Five game-feel patterns | (1) Visual IS the mechanism; (2) Live HUD counters; (3) One legible interaction per scene; (4) 3D when spatial teaches, 2D when not; (5) Phosphor fade on 2D canvases |
| Performance | 60fps on M1 with 4× CPU throttle. One heavy Three.js scene at a time. IntersectionObserver pauses RAF. |
| Accessibility | `prefers-reduced-motion` freezes scenes; keyboard controls; high-contrast respect |
