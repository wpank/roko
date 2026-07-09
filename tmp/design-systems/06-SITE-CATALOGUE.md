# Site Catalogue — 14 HTML Iterations

A look/feel reference for every iteration, organized by lineage.

---

## Lineage Map

Two evolutionary branches, converging over time:

```
rosedust-prototype → rosedust-v2 → rosedust-v3 → rosedust-v4 → rosedust-v5 → rosedust-v6
                                         ↘
                          nunchi → nunchi_1 → nunchi_2 → nunchi_3 → nunchi_4 → nunchi5 → nunchi_5
```

**Rosedust branch** = "chassis / instrument panel" aesthetic with 2D canvas scenes + Three.js hero.
**Nunchi branch** = "glass / orbit / particle" aesthetic with full Three.js scenes + HUD overlays.

---

## File-by-File Catalogue

### rosedust-prototype.html

**Mood:** Terminal existentialism. CRT phosphor. "A boot screen that slowly reveals itself."
**Sections:** 9 (Boot → Hero → Moat → Loop → Gate → VCG → Network → Outro)
**Three.js:** 1 scene — HDC Vector Field (2500 particles, custom ShaderMaterial with per-particle phosphor glow, "bind" lines between nearby particles via random nearest-neighbor sampling)
**Interactive:** Loop auto-cycle with click/hover. Gate PE slider (logistic tier splits). VCG auto-mutating bids. Network slider (1–1000 agents as ASCII character grid). Scroll reveal.
**Unique:** Custom vertex+fragment ShaderMaterial (not just MeshBasicMaterial). `pow(random, 2.5)` color bias. Bind lines with sine-envelope alpha. ASCII agent grid with 5 Unicode circle chars. Three.js r128 (older CDN).
**Library:** `three.js r128` via cdnjs.

---

### rosedust-v2.html

**Mood:** Terminal-existential. Isometric cutaway visualization. "A cognitive system booting itself."
**Sections:** 8 (Boot → Hero → Moat → Loop → Gate → VCG → Swarm → Outro)
**Three.js:** 2 scenes — Hero Stack (6 BoxGeometry plates + binary tree + knowledge particles) + Swarm (1000 instanced IcosahedronGeometry agents + bezier arcs + motes)
**Interactive:** Hero drag-orbit + raycaster hover tooltip on plates. Loop click/hover phases + auto-cycle. Gate PE slider. VCG auto-mutating. Swarm slider (1–1000 agents). Boot sequence.
**Unique:** Knowledge particle arc from Roko plates to Korai tree blocks (smoothstep + sine offset). Live FPS+triangle counter. Uptime counter in nav. Arc "traveling head" technique (`|u - head| * 4` intensity falloff). Golden-angle spiral agent placement.

---

### rosedust-v3.html

**Mood:** Terminal.existential v2. "Scientific instrument from a dystopian research lab."
**Sections:** 9 + Outro (Hero → Moat → Loop → Gate → VCG → HDC → Network → Korai → Demo)
**Three.js:** 3 scenes — Hero Stack (same 6 plates + tree + particles, now with chip drag-to-remove) + Swarm (+ click-to-spawn/remove) + Korai Lattice (40 instanced glass blocks + CatmullRomCurve3 pulse)
**Interactive:** Hero layer chips (drag-to-remove, click toggle, cost model HUD). Gate draggable threshold dividers on canvas + slider. VCG click-to-boost columns + mini grid. HDC Playground (FNV-1a + xorshift32, 10240-bit fields). Swarm click-to-spawn/remove + slider. Demo triptych (cold/warm scripted replay). Korai scroll-parallax camera.
**Unique:** `SceneReg` visibility registry (hot-set gating). HDC Playground (entirely new). Demo Triptych (cold vs warm cost comparison). Korai scroll-parallax. Swarm auto-scale FPS guard. VCG second-price flash. Loop zoom-to-sub-mechanism per phase.
**Evolution:** First appearance of chassis component (corner screws, header/footer bars, LED pulse).

---

### rosedust-v3 (1).html

**Mood:** Identical to rosedust-v3. Copy/variant with same content.
**Sections:** Same 9 sections.
**Three.js:** Same 3 scenes.
**Notes:** Likely a saved copy during iteration.

---

### rosedust-v4.html

**Mood:** Same chassis aesthetic. Internal label "rosedust-v3.0" / title "v3".
**Sections:** 9 + Outro — same layout as rosedust-v3
**Three.js:** 3 scenes — Hero Stack + Swarm + Korai Lattice (identical to v3)
**Interactive:** All same interactives as v3 (chip drag, HDC, demo triptych, etc.)
**Unique:** Fixed bottom-right perf badge (FPS · TRIS · SCENE NAME). Scroll progress bar 1px. 1px rose scroll progress.
**Notes:** Minimal delta from v3 — mostly polish.

---

### rosedust-v5.html

**Mood:** "Cinematic, CRT-terminal gothic. Dying terminal in deep space."
**Sections:** 14 (Boot → Hero → Thesis Band → Amnesia Split → Moat → Route Band → Gate → Remember Band → HDC → Lattice → Compound Band → Swarm → Outro)
**Three.js:** 3 scenes — Hero (rose-dust particle field + orbiting sigil) + Gate Freeway (isometric 3-lane) + Lattice (glass-prism block tree) + Swarm (Fibonacci-sphere constellation)
**Interactive:** Same core set (HDC, Swarm slider, Gate PE). Thesis bands as interstitial separators. Amnesia split (dead vs live agent trace comparison).
**Unique:** Boot sequence with `.reveal` fade-up animation. Thesis bands as structural rhythm. "Amnesia" comparison (dead trace / live trace side-by-side). Perf HUD (bottom-right). SVG compounding curve chart in Moat.
**Evolution:** More editorial structure with thesis bands between sections.

---

### rosedust-v6.html

**Mood:** "Industrial brutalism with CRT/VHS phosphor-screen atmosphere. Decommissioned military console."
**Sections:** 16 (Hero → Cold Open → Problem → Category → Proof → Runtime → Cascade → Compose → Memory → Network → Chain → Demo → Compliance → Moat → CTA → Footer)
**Three.js:** Multiple (Hero coordination plane, MAST bars, 8-phase orbit, Freeway, VCG, Swarm, Korai Lattice)
**Interactive:** Full suite: all previous interactives + animated dual cost meter (Canvas 2D). Proof section with 3-mechanism breakdown. Category as 4-column market map.
**Unique:** Most comprehensive version. All chassis patterns mature. Compliance section. Market map. Cost meter. Most sections have both 3D and 2D canvas.
**Evolution:** Peak rosedust — maximum content density and interactive variety.

---

### nunchi.html

**Mood:** "Glass / orbit" — lighter than rosedust. ROSEDUST glassmorphism over Three.js canvases.
**Sections:** 14 (Hero → Problem → CLI Demo → Primitives → Protocols → Cost → Use Cases → Why Now → + more)
**Three.js:** 11 scenes — Hero Two-Plane (control+execution wireframe grids, central icosahedron, 80 agents, 44 signals, citation arcs) + MAST Bars + Duality (Signal↔Pulse graduation) + Loop (8-phase with trail) + PPC + Demurrage + Vitality + Swarm + Fractal Zoom + Capability (3-ring intersection) + Chain Lattice
**Interactive:** Hero drag-orbit. Fractal zoom (mouse wheel + drag). Capability chips (send pulse, allowed/blocked). Demurrage buttons (retrieved, cited, gatepass, surprised, antiknow). Vitality slider (phase control). Swarm slider (1–1000 with pheromone grid). Sessions sigmoid slider.
**Unique:** `createScene()` factory function (shared renderer/camera/ResizeObserver pattern). `window.NUNCHI_*` globals bridge for multi-module scripts. `inViewport()` guard. 11 different Three.js scenes (most of any file). Pheromone field simulation. DPR-aware 2D canvas drawing.
**Evolution:** Paradigm shift — from chassis/2D-canvas to glass/3D-everything.

---

### nunchi_1.html

**Mood:** Same as nunchi.html — glass/orbit paradigm.
**Sections:** 14 — same structure as nunchi.html
**Three.js:** 11 scenes — identical scene set to nunchi.html
**Interactive:** Same interactive set.
**Notes:** Near-identical to nunchi.html. Same `createScene()` factory.

---

### nunchi_2.html

**Mood:** Same glass/orbit. 14 sections. Slightly different hero.
**Sections:** 14 — same structure
**Three.js:** 7 scenes — Hero Orbit (5 elliptical rings, squashed diamond octahedra, 220 dust points with vertexColors, 80 square plane sprites) + Loop (with trail + emitted signals) + PPC + Demurrage + Vitality + Swarm (InstancedMesh + pheromone grid + InsightStore column) + Chain
**Interactive:** Hero drag-orbit (pointerdown/up/move). Same set of sliders/buttons as nunchi_1 but fewer scenes.
**Unique:** Ambient floating squares (70 DOM divs with interval-based CSS transforms). Different hero (orbit rings vs two-plane).

---

### nunchi_3.html

**Mood:** More refined glass/orbit. Transition to smoother mouse interaction.
**Sections:** 24 sections (expanded content, more editorial)
**Three.js:** 3 scenes — Hero Orbit (5 concentric rings, 10 orbiting diamonds, 420 dust particles) + Cognitive Loop (8-phase) + Chain Prism Lattice
**Interactive:** Hero pointermove lerp (NO drag required — smooth follow). Sessions sigmoid slider. Vitality slider. Swarm count slider (1→1000). Phase tiles (clickable grid → Three.js scene update).
**Unique:** Pointermove lerp paradigm (factor 0.06) replaces drag-orbit for hero. 24 sections (most content). Loading curtain with rotating diamond.
**Evolution:** Mouse interaction paradigm shift — from grab-to-drag to passive mouse follow.

---

### nunchi_4.html

**Mood:** Same as nunchi_3. Identical structure.
**Sections:** 24 sections
**Three.js:** Same 3 scenes (Hero Orbit, Loop, Chain)
**Interactive:** Same interactive set as nunchi_3.
**Notes:** Copy/polish variant of nunchi_3.

---

### nunchi5.html (no underscore)

**Mood:** Same glass/orbit. 24 sections.
**Sections:** 24 sections — same structure as nunchi_3/4
**Three.js:** Same 3 scenes — Hero Orbit (identical ring/diamond setup) + Loop + Chain
**Interactive:** Same. Pointermove lerp. Sliders. Phase tiles.
**Notes:** Incremental polish from nunchi_4.

---

### nunchi_5.html (with underscore)

**Mood:** Most polished nunchi variant. "The reference implementation."
**Sections:** 24 sections
**Three.js:** Same 3 scenes + hero with stage toggle (chaos ↔ coordination)
**Interactive:** All nunchi_3 interactives + telemetry sidebar (fixed right panel, hidden <1280px, live-updating metrics). Definition term tooltips (`<span class="defterm" data-def="...">term</span>`). Hero stage toggle buttons. Ambient floating particles.
**Unique:** Telemetry sidebar. Definition term hover tooltips. Stage toggle (chaos ↔ coordination mode on hero particles). Most editorial content.
**Evolution:** Peak nunchi — maximum polish and content density.

---

## Evolution Summary

### Visual Paradigms

| Era | Files | Hero Pattern | Mouse Interaction | Sections | Aesthetic |
|-----|-------|-------------|-------------------|----------|-----------|
| Prototype | rosedust-prototype | ShaderMaterial particles | Scroll only | 9 | Phosphor CRT |
| Chassis | rosedust-v2 through v6 | 3D stack cutaway | Drag-orbit | 8–16 | Instrument panel / brutalist |
| Glass/Orbit early | nunchi, nunchi_1 | Two-plane wireframe | Drag-orbit | 14 | Glass panels over 3D |
| Glass/Orbit mid | nunchi_2 | Elliptical orbit rings | Drag-orbit | 14 | Glass + floating squares |
| Glass/Orbit late | nunchi_3 through nunchi_5 | Concentric orbit rings | Pointermove lerp | 24 | Glass + ambient particles |

### Interactive Element Evolution

| Feature | First Appeared | Matured In |
|---------|---------------|------------|
| PE slider (gate routing) | rosedust-prototype | rosedust-v3 (+ draggable canvas dividers) |
| Agent count slider (1-1000) | rosedust-prototype | rosedust-v3 (+ click-to-spawn/remove) |
| VCG auction auto-mutate | rosedust-prototype | rosedust-v3 (+ click-to-boost) |
| Boot sequence | rosedust-prototype | rosedust-v5 (+ reveal animation) |
| HDC Playground | rosedust-v3 | rosedust-v6 |
| Demo Triptych (cold/warm) | rosedust-v3 | rosedust-v6 |
| Layer chip drag-to-remove | rosedust-v3 | rosedust-v4 |
| Drag-orbit camera | rosedust-v2 | nunchi_2 |
| Pointermove lerp camera | nunchi_3 | nunchi_5 |
| Fractal zoom (scroll+drag) | nunchi_1 | nunchi_1 |
| Capability ring pulses | nunchi_1 | nunchi_1 |
| Demurrage buttons | nunchi_1 | nunchi_2 |
| Telemetry sidebar | nunchi_5 | nunchi_5 |
| Definition term tooltips | nunchi_5 | nunchi_5 |
| Stage toggle (chaos/coord) | nunchi_5 | nunchi_5 |
| Phase tiles (click→scene) | nunchi_3 | nunchi_5 |
| Sessions sigmoid slider | nunchi_3 | nunchi_5 |
| Vitality phase slider | nunchi_1 | nunchi_5 |
