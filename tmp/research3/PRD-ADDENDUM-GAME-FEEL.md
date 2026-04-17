# PRD Addendum: Game-Feel Miniatures

**Supplement to:** PRD-NUNCHI-LANDING.md
**Date:** 2026-04-21
**Status:** specifies the 5 miniatures not in v2 prototype, plus the pattern language v2 establishes

---

## 0. What v2 establishes (use as the pattern for the rest)

The v2 prototype (`rosedust-v2.html`) builds three full game-feel miniatures and establishes five patterns every other section must follow:

### Pattern 1: Every section has a **primary visual object** that IS the mechanism.

The visual doesn't illustrate the concept — it executes it. The hero's 3D stack isn't a diagram of Roko; it's Roko. The freeway isn't a chart; it's cognitive gating happening. The swarm isn't a graph of a network; it's the network. Text becomes annotation around the object, never the main vehicle.

### Pattern 2: Every mechanism has a **live state counter** somewhere in the HUD.

The freeway shows "THROUGHPUT · N ticks" and "SAVED VS ALWAYS-FRONTIER · $N.NN" — counters that accumulate as you watch. The swarm shows InsightStore size, gate-pass %, cost/episode. These numbers tick in real time. They're how you know you're watching something alive, not a rendered animation.

### Pattern 3: Every scene has **one legible interaction** — not many.

Hero: drag to orbit, hover for layer info. Freeway: one slider (prediction error). Swarm: one slider (agent count). Do not cram 4 controls into one scene. One axis of control per miniature. The VC plays one thing, sees it do something, moves on.

### Pattern 4: **Three.js when the spatial/depth dimension teaches something; 2D canvas when not.**

Hero (spatial stack), Swarm (3D graph of nodes in space) — Three.js earns it. Loop (orbital diagram), Freeway (top-down flow), VCG (bar chart pit) — 2D canvas is cleaner, renders faster, reads better. Don't force 3D where 2D tells the story better. The rule: if rotating the camera would reveal new information, use 3D. If not, use 2D.

### Pattern 5: **Animated phosphor fade instead of full clear on 2D canvases.**

All 2D canvases paint over with `rgba(6, 6, 8, 0.18-0.22)` each frame rather than `clearRect`. This creates trails — the CRT phosphor afterimage. Any moving element leaves a rose-colored ghost that fades over ~5-10 frames. This is the single effect that makes 2D canvas work in ROSEDUST; don't skip it.

### Color and pulse rules specific to the 3D scenes

- **Materials**: `MeshPhongMaterial` with `emissive` set to the ROSEDUST token, `emissiveIntensity` in 0.15-0.6, `transparent: true, opacity: 0.65-0.9`. Glass-like, never plastic.
- **Edge highlights**: always add `EdgesGeometry` + `LineBasicMaterial` on top of box/mesh geometry. The edge lines are where rose gets to sing.
- **Lighting**: one warm rose directional key light, one cool indigo fill, one rose point-light as rim. Never white light.
- **Fog**: `FogExp2(0x060608, 0.02-0.035)` on every scene. Depth disappears into the void. Always.
- **Particle additive blending**: every particle system uses `AdditiveBlending` + `depthWrite: false` + `transparent: true`. This is what gives the phosphor glow.

---

## 1. Moat compounding chart — IN v2 AS SVG

The compounding-vs-linear divergence chart is built in v2 as an SVG. Port that exact SVG to React as a component — it doesn't need to be interactive, it needs to animate on scroll-into-view:

- The dashed "frontier-only · linear" line draws itself left-to-right over ~2s on first reveal.
- The rose "nunchi · compounding" curve draws after the dashed line, also left-to-right, over ~2.5s with slight ease-out.
- The gradient fill underneath the rose curve fades in during the last 500ms of the draw.
- The 5 pink dots along the rose curve pop in staggered (100ms apart) after the curve completes.
- The "230pt · delta by 100K" callout fades in last.
- All animated via `SVGGeometryElement.getTotalLength()` + stroke-dasharray trick.

Total animation duration: ~4s on reveal. Scroll the page past it, scroll back — replays.

---

## 2. Korai section (§06) — Game-feel spec

**What v2 is missing:** a miniature of the chain. This is the one section the v1 spec downweighted, but it needs a full game-feel miniature for the moat story to close.

### 2.1 The primary visual: "The Lattice"

A 3D chain tree rendered as **glass prisms**. Each prism is a block. Blocks hang from a root at the top and branch downward (this is Kauri BFT's dissemination shape).

- **Geometry**: each block is a translucent `BoxGeometry(0.8, 0.8, 0.8)` with `MeshPhysicalMaterial` — `transmission: 0.8, thickness: 0.5, roughness: 0.1, ior: 1.5` — actual glass. Emissive indigo (`P.dream`) at low intensity; gets brighter as insights accumulate inside.
- **Arrangement**: ternary tree, 4 levels. Level 0 = root, level 3 = leaves. 1 + 3 + 9 + 27 = 40 blocks total.
- **Connections**: thin rose-deep lines between parent/child blocks. Lines have a traveling "block-pulse" sprite on them — a small bright rose dot traveling root→leaves every 400ms (Korai's block time).
- **Insights inside blocks**: each block contains 3-8 smaller glowing motes (tiny spheres of various ROSEDUST colors). Motes drift slowly inside the glass cube like fireflies in a jar. When a new insight is "deposited," a new mote appears in a random leaf block with a pop animation.

### 2.2 HUD elements

Absolute-positioned over the canvas:

```
TOP LEFT:    BLOCK                   #0x0011a7e
             ────                    400ms · single-slot finality

TOP RIGHT:   ISFR                    3.72% ▲
             14-DAY MEDIAN           +0.03 since last tick

BOT LEFT:    INSIGHTSTORE            N entries
             6 TYPES · DECAYING      → next prune in 4m 12s

BOT RIGHT:   TPS                     12,847
                                     kauri bft · O(n) messages
```

The block number increments every 400ms. The ISFR rate jitters slightly with each block (mean-reverting random walk around 3.72%, ±0.05 range).

### 2.3 Yield perps gap visual (below the lattice)

A single arresting SVG. 1000×140 viewport:

```
Two horizontal bars, drawn in the same coordinate system but one labeled "log scale"

TRADFI IR DERIVATIVES  ████████████████████████████████████████████  $668T
                       └────────────────────────────────────────────┘
                                                                     log(notional)

ON-CHAIN IR PRODUCTS   ▏                                              $100M
                       └┘

The gap between the end of the second bar and the end of the first is ENORMOUS.
Text overlay: "6 orders of magnitude — nunchi 1st app" in bone, centered.
```

Animate bars drawing left-to-right on scroll-into-view. Let the gap speak.

### 2.4 Three.js implementation notes

- The lattice rotates slowly (0.003 rad/s on Y). Drag to orbit. Scroll within section zooms slightly.
- The block-pulses traveling along edges are implemented as small `THREE.Points` that lerp along the parent-child line segment, with a trail that fades.
- The insight motes drifting inside cubes are instanced `THREE.Points` parented to each block, animated with tiny sinusoidal jitter.
- Performance: 40 glass blocks with `MeshPhysicalMaterial` and transmission is expensive. If it stutters, drop to `MeshStandardMaterial` with `transparent: true, opacity: 0.4` — 95% of the look, 20% of the cost.

---

## 3. HDC Playground (new addition — v1 spec had it, v2 doesn't)

This is the interactive that wins Twitter. Ship it in v1 of the real build.

### 3.1 The primary visual: "Interference"

Two side-by-side 64×32 grids. Each grid represents a compressed view of a 10,240-bit HDC vector (showing 2,048 bits). Lit cells = 1, dark cells = 0. Rendered as canvas or SVG (SVG is fine for 2,048 cells × 2 grids).

Above each grid: a text input. The input's text is hashed deterministically (FNV-1a or similar) into the 2,048-bit pattern. Same text → same pattern, always.

Below the two grids, a third region: the **interference pattern** — the XOR of the two grids, visualized. Cells where both grids agree are dim (rose-deep). Cells where they disagree are bright (rose-bright). The density of bright cells = Hamming distance.

### 3.2 Interaction

- Left input placeholder: "verify before commit"
- Right input placeholder: "verify before position"

As you type, both grids update live (debounced 100ms). The XOR interference grid recomputes. A readout below shows:

```
HAMMING DISTANCE        847 / 10,240    (8.3%)
SIMILARITY              0.917
CROSS-DOMAIN TRANSFER   ✓ TRIGGERED
```

When similarity > 0.85, a badge fires with a rose-bright glow pulse and the following sentence types out below (character-by-character at ~30ms/char):

> *"These encode as structurally equivalent patterns. Knowledge from the trading agent is now available to the coding agent's context window."*

### 3.3 Preset pairs (VC demo mode)

Four quick buttons above the inputs let the presenter show canned pairs:

- `verify before commit` ⇔ `verify before position` → high similarity (structural equivalence)
- `optimize compile` ⇔ `oat milk latte` → very low similarity (control)
- `reduce retry backoff` ⇔ `lower position size after drawdown` → medium-high (noisy-but-real)
- `bind(high-surprise, escalate-model)` ⇔ `bind(high-vol, reduce-leverage)` → very high (HDC bind op)

Clicking a preset fills both inputs and auto-scrolls the visualization.

### 3.4 Why this matters for the VC

HDC is the weirdest, most-provable-on-demand piece of the tech stack. A VC who's seen a thousand AI pitches has never seen two text inputs interfering with each other as bit-fields. This is the visual that goes on their partner meeting slide.

---

## 4. End-to-end Demo section (§07) — Game-feel spec

### 4.1 The primary visual: "Telemetry Triptych"

Three panels side-by-side, 45%/30%/25% width split:

**LEFT (45%) — the terminal.** xterm.js (or asciinema player) playing the scripted coding-agent run. ROSEDUST themed. 80 columns wide. Cursor blinks. This is where the text lives.

**CENTER (30%) — the loop monitor.** Live-updating canvas showing the 6-phase orbit from §02 (the Loop section), but in miniature. As the terminal reports which phase it's in, the corresponding node in the orbit lights up. Current tier (T0/T1/T2) is shown as a badge. Cost accumulates as a number below the orbit.

**RIGHT (25%) — the chain monitor.** A simplified version of the Korai lattice from §06. When the terminal emits an `insight.insert(...)` line, a new glowing mote spawns in a leaf block with a pop animation. A running counter shows total insights deposited this session.

All three panels update *together*. The terminal is the driver; the other two are slaves.

### 4.2 Scripted run

The transcript from the v1 PRD (§4.8) is good — use it unchanged. Pacing:

- Characters stream at ~50ms/char
- Pauses of ~600ms at each `[γ tick]` line (gives the other two panels time to react)
- Pause of ~1500ms after the `FAIL` on the test gate (the tension beat — the VC thinks "oh no")
- Pause of ~2000ms at the end after "knowledge deposited: 1 insight (staged)"

Total run time: ~70-90 seconds. Too fast for comfortable watching; the VC should lean in. Play/pause/restart/speed (1×/2×/0.5×) controls below.

### 4.3 After-the-fact annotation

When the run completes, a panel slides up from below (3s delay, 500ms transition) containing the 4-bullet "this is what just happened" from v1 PRD §4.8. Plus one new line at the top in bone:

```
VS. A STATELESS FRAMEWORK: this task would have cost 7× more
and deposited nothing for the next agent.
```

### 4.4 The "play again" button

After the run completes, a "RUN IT AGAIN — WATCH IT START SMARTER" button appears. On click, the terminal replays, but the scripted transcript is slightly different:

- OBSERVE shows `memory_hit: 0.95` instead of 0.78 (higher — it remembers from last run)
- GATE routes to T0 on the first tick (cheap-lane — it recognized the task)
- No gate failure this time — the insight from the first run prevented the test-gate miss
- Total cost: $0.008 instead of $0.043 (5× cheaper)
- Chain panel shows the insight promoting from STAGED to PERMANENT

This is the kill shot. Session N+1 is categorically better. The VC watches it happen.

---

## 5. VCG (§04) — Upgrade path from v2

v2's VCG pit is a 2D canvas with bid-bars rising and a second-price line. This is good. The upgrade path for production:

### 5.1 Add tension animations on bid-change

When bids mutate (every 2.2s), don't just linearly interpolate — stagger the changes. Bidders whose bids went UP animate first (150ms each, rose-bright flash on the bar top). Bidders whose bids went DOWN animate after (300ms delay, rose-dim subdued drop). This creates a narrative — "Neuro wants more; Playbook is yielding."

### 5.2 Context window bar — add token-count labels

In each slice of the context window bar, show the token count when slice width > 50px: "10,880 tok". Currently only the bidder name shows. Token count is the number the VC cares about.

### 5.3 Make bidder hover show "what this bids on"

Hovering the bid-bar reveals a tooltip card with:
- Bidder name
- 1-sentence description of what this subsystem contributes ("retrieves HDC-similar past episodes" for Neuro)
- The 3 most recent entries it won context space for (fictional titles, rose-dim)

---

## 6. Loop (§02) — Upgrade path from v2

v2's loop is a 2D canvas orbital with nodes arranged in a circle. Good. Production upgrades:

### 6.1 Flow particles on the spokes

When a phase is active, particles flow FROM the central tick core OUT to that phase's node, along the spoke. When the phase completes (right before auto-advance), particles flow BACK to the core. This makes "dispatching work" and "gathering results" visible.

### 6.2 Inter-phase flow on the orbit ring

The orbit ring itself should have rose-bright particles traveling clockwise, synchronized with the auto-advance. As phase N hands off to phase N+1, a bright packet of particles detaches from node N and travels along the ring to node N+1. It arrives just as the active indicator shifts.

### 6.3 Sub-mechanism callouts

When a phase is active, small text labels fade in AROUND its node describing its sub-mechanisms. E.g. when ASSEMBLE is active:

```
            8 BIDDERS
                │
           ┌────┴────┐
     VCG   │ ASSEMBLE │   SECOND-PRICE
           └────┬────┘
                │
          32,000 TOK
```

Each label fades in at 200ms delay, positioned around the node. Fades out when the phase becomes inactive.

---

## 7. Gate/Freeway (§03) — Upgrade path from v2

v2's freeway is already quite good. Small polish additions:

### 7.1 Lane-change animations

When a vehicle spawns and its tier is determined, it should briefly move down the LEFT edge of the screen before snapping into its lane. Currently they appear already in-lane. Adding a 300ms lane-sort animation sells the "routing" story.

### 7.2 Probe visualization in the left margin

Between the spawn indicator and the lane entrance, show 16 tiny rose dots representing the 16 probes firing on each tick. When a vehicle spawns, a visible "signal" travels from the probe column to the vehicle's lane-selection point. This makes it clear that probes drive the routing.

### 7.3 Cost ticker on each lane

Small running cost counter at the right edge of each lane, showing cumulative $ spent in that lane:

```
T0 ──────────────────── $0.000
T1 ──────────────────── $0.412
T2 ──────────────────── $2.304
```

The gap between T0 and T2 is the visceral story.

---

## 8. Swarm (§05) — Upgrade path from v2

v2's swarm works. Two additions for production:

### 8.1 "Knowledge pressure" aura

Each agent node has a small, semi-transparent halo whose size represents how much stored knowledge the network has relevant to that agent. As the slider goes up and InsightStore grows, all halos grow proportionally. Visualization of the invisible scaffold around each agent.

### 8.2 Crunchy transition: 1-agent to 1000-agents

When you drag the slider from 1 → 1000 fast, the current v2 just pops in new agents instantly. Upgrade: new agents fade in over 100-300ms each with a tiny rose flash (birth event). Removed agents fade out similarly. This makes the scaling feel consequential rather than instant.

### 8.3 Domain clustering

Agents aren't uniform. Give them typed tags (coding / trading / research / security). Cluster them loosely in 3D space by type — coding agents top-left, trading bottom-right, etc. Knowledge-transfer arcs between DIFFERENT types glow bone instead of rose, marking cross-domain transfer (the headline result). Hovering an arc shows:

```
FROM:  trading-agent-0x7f · "reduce position on drawdown"
TO:    coding-agent-0x2b  · applied as "reduce retry count on repeated failure"
HDC:   cosine 0.89 · bind(high_uncertainty, pullback)
```

This is the single most important thing the swarm can teach: cross-domain transfer via HDC structural similarity. Make it legible.

---

## 9. Performance budget (all scenes combined)

Production targets — these are tight but achievable with the optimizations noted:

| Scene | Target triangle budget | Max instances | Fallback trigger |
|-------|------------------------|---------------|------------------|
| Hero stack | 3K triangles, 200 particles | N/A | If FPS < 40 on 2021 M1 MacBook Air with CPU 4× throttle, drop knowledge particles |
| Loop orbital | 2D canvas (no triangles) | N/A | If FPS < 40, drop flow particles, keep nodes static |
| Freeway | 2D canvas (no triangles) | 40 max vehicles on screen | If FPS < 40, cap at 20 vehicles |
| VCG | 2D canvas (no triangles) | N/A | Already cheap |
| Swarm | 1K agents × ~80 triangles = 80K, + 40 arcs × 23 segs | 1000 instances | If FPS < 30, cap agent count at 500 and warn in HUD |
| Korai lattice | 40 glass blocks × ~100 triangles + 120 motes = 4K | N/A | Swap MeshPhysicalMaterial for MeshStandardMaterial |
| HDC playground | SVG — trivial | N/A | Never a problem |
| Demo triptych | xterm + 2 small canvases | N/A | Already cheap |

**Only one heavy Three.js scene renders at a time.** Use IntersectionObserver to pause `requestAnimationFrame` on scenes not in view. This is critical — without it, 4 concurrent Three.js renderers will thrash.

```js
// pattern
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

---

## 10. Accessibility addendum

The game-feel visuals raise new accessibility concerns:

- **Every section must have a "still-image" fallback.** If `prefers-reduced-motion: reduce`, each scene renders one frame and freezes. Still beautiful. Not animated. The v2 already freezes flicker; extend this to all scene motion.
- **Keyboard controls.** The slider for PE (Gate) and agent count (Swarm) must be keyboard operable (arrow keys). The orbital loop canvas should have arrow keys for next/prev phase. The HDC inputs are already keyboard-accessible.
- **Live regions.** HUD counters (ticks, saved, etc.) update frequently; they should NOT be wrapped in `aria-live` regions because that would spam screen readers. Instead, provide a single summary button ("show current state") that reveals a static snapshot.
- **High-contrast mode.** If the user has a high-contrast OS setting, bump rose_bright and bone opacity to 100%, remove scanlines, increase font weight. Do not change the color scheme — ROSEDUST remains — but push the signal.

---

## 11. Handoff checklist

When handing this to Claude Code for the real build:

- [ ] Read `/mnt/user-data/outputs/rosedust-v2.html` in the browser. Play with all three miniatures. The three patterns it establishes (primary object IS the mechanism, live HUD counters, one legible interaction) must be honored in every other section.
- [ ] Port v2's Hero stack, Freeway, and Swarm to `@react-three/fiber` + canvas-as-component. Keep the exact feel; the vanilla JS can be wrapped in refs.
- [ ] Build the Korai lattice (§2) — this is the highest-impact new scene.
- [ ] Build the HDC playground (§3) — this is the highest-share-per-minute-of-effort scene.
- [ ] Build the Demo triptych (§4) — this is the kill-shot scene.
- [ ] Apply upgrades (§§5-8) as budget allows. The v2 versions are passable if time is short.
- [ ] Enforce the performance budget (§9) BEFORE polish. A jittery hero makes the site feel cheap no matter how beautiful the stills are.
- [ ] Final pass: scroll the full page on a 2021 M1 MacBook Air with Chrome CPU throttled 4×. The whole experience at 60fps is the quality bar. 55fps is acceptable if sustained. Below 45fps, reduce quality.

---

## 12. What the VC sees in 10 minutes (updated with v2 visuals)

| Time | What they see | What they feel |
|------|----------------|-----------------|
| 0:00 | Boot sequence | "Oh. This is different." |
| 0:30 | 3D stack rotating, knowledge deposits flowing from Roko down into Korai tree | "I can see the system." |
| 2:00 | Compounding curve divergence animates | "The gap widens. Fast." |
| 3:30 | Orbital loop, 6 phases, active one glowing | "Five of these six, competitors don't have." |
| 5:00 | Freeway: tokens flowing, 80% taking the free lane, $ counter climbing on T2 saved | "Holy shit, look at the savings." |
| 6:30 | VCG pit: 8 bars rising/falling, second-price line, context window reshaping | "The auction is doing something real." |
| 8:00 | Swarm: slide from 47 to 1000 agents, watch cross-domain arcs multiply | "The network compounds. I can see it." |
| 9:00 | Demo triptych: terminal, orbit monitor, chain monitor — all lit together | "This is a working system." |
| 10:00 | Outro. "Three of four quadrants are populated." | "They're in the empty quadrant." |

That's the pitch. Built visual-first.
