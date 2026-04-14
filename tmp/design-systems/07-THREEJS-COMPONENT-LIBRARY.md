# Three.js Modular Component Library

Reusable scene patterns, interactive elements, and infrastructure extracted from 14 site iterations. Each component includes full implementation code, parameters, and integration notes.

---

## Table of Contents

1. [Infrastructure](#1-infrastructure)
2. [Hero Scenes](#2-hero-scenes)
3. [Data Visualization Scenes](#3-data-visualization-scenes)
4. [Interactive Controls](#4-interactive-controls)
5. [2D Canvas Scenes](#5-2d-canvas-scenes)
6. [Atmospheric & UI Components](#6-atmospheric--ui-components)

---

## 1. Infrastructure

### 1A. Shared Palette Object

Every Three.js scene should reference this single palette constant. Mirrors CSS `:root` variables.

```javascript
const P = {
  void:       0x060608,
  raised:     0x0c0a0e,
  rose:       0xaa7088,
  roseBright: 0xcc90a8,
  roseDim:    0x7a5060,
  roseDeep:   0x3a2030,
  roseEmber:  0x482838,
  bone:       0xc8b890,
  boneBright: 0xd8c8a0,
  boneDim:    0x8a7a5a,
  dream:      0x585878,
  dreamBright:0x9494b4,
  dreamDim:   0x383858,
  success:    0x70887a,
  warning:    0xaa8855,
  textPrim:   0x988090,
  textDim:    0x584858,
};

function rgb(hex, a) {
  const r = (hex >> 16) & 255, g = (hex >> 8) & 255, b = hex & 255;
  return a == null ? `rgb(${r},${g},${b})` : `rgba(${r},${g},${b},${a})`;
}
```

### 1B. Scene Factory (createScene)

Shared pattern for creating Three.js scenes with automatic resize, IntersectionObserver lazy-start, and DPR capping.

```javascript
function createScene(canvasId, { fov = 45, near = 0.1, far = 200, fog = null } = {}) {
  const canvas = document.getElementById(canvasId);
  const scene = new THREE.Scene();
  if (fog) scene.fog = new THREE.FogExp2(fog.color || 0x060608, fog.density || 0.025);

  const camera = new THREE.PerspectiveCamera(fov, canvas.clientWidth / canvas.clientHeight, near, far);

  const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
  renderer.setSize(canvas.clientWidth, canvas.clientHeight, false);
  renderer.setClearColor(0x060608, 0);

  // Auto-resize
  const ro = new ResizeObserver(() => {
    const w = canvas.clientWidth, h = canvas.clientHeight;
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
    renderer.setSize(w, h, false);
  });
  ro.observe(canvas);

  return { scene, camera, renderer, canvas };
}
```

### 1C. Scene Registry (Visibility Gating)

Only run animation loops for scenes currently in the viewport. Prevents offscreen rendering.

```javascript
const SceneReg = {
  hot: new Set(),
  tris: {},
  current: null,

  setTris(name, n) { this.tris[name] = n; },
  setCurrent(name) { this.current = name; },
};

function watchVisibility(sectionId, sceneName, onHot, onCold) {
  const el = document.getElementById(sectionId);
  let isHot = false;
  new IntersectionObserver(entries => entries.forEach(e => {
    const nowHot = e.isIntersecting && e.intersectionRatio > 0.1;
    if (nowHot && !isHot) {
      isHot = true;
      SceneReg.hot.add(sceneName);
      SceneReg.setCurrent(sceneName);
      onHot?.();
    } else if (!nowHot && isHot) {
      isHot = false;
      SceneReg.hot.delete(sceneName);
      onCold?.();
    }
  }), { threshold: [0, 0.1, 0.25, 0.5] }).observe(el);
  return () => isHot;
}

// Usage in animation loop:
function animate() {
  requestAnimationFrame(animate);
  if (!SceneReg.hot.has('myScene')) return;
  // ... render
}
```

### 1D. InViewport Guard (Simpler Alternative)

For simpler setups without the full registry:

```javascript
function inViewport(el) {
  const r = el.getBoundingClientRect();
  return r.bottom > 0 && r.top < window.innerHeight;
}

// In animation loop:
function tick() {
  requestAnimationFrame(tick);
  if (!inViewport(canvas)) return;
  // ... render
}
```

### 1E. Mouse Interaction Patterns

**Pattern A: Pointermove Lerp (passive follow, no click required)**
Best for hero scenes. Used in nunchi_3 through nunchi_5.

```javascript
let mx = 0, my = 0;       // current (lerped)
let tmx = 0, tmy = 0;     // target (raw)
window.addEventListener('pointermove', e => {
  tmx = (e.clientX / window.innerWidth - 0.5) * 2;   // -1 to 1
  tmy = (e.clientY / window.innerHeight - 0.5) * 2;
});
// In animation loop:
mx += (tmx - mx) * 0.06;
my += (tmy - my) * 0.06;
// Apply to scene:
root.rotation.y = mx * 0.4 + t * 0.04;
root.rotation.x = my * 0.3;
camera.position.x = mx * 1.5;
camera.position.y = my * 1.2;
```

**Pattern B: Drag Orbit (click-and-drag to rotate)**
Best for explorable 3D scenes. Used in rosedust-v2 through v4, nunchi_1/2.

```javascript
let isDragging = false, lastX = 0, lastY = 0;
let targetRotX = 0.35, targetRotY = 0.5;
let curRotX = targetRotX, curRotY = targetRotY;

canvas.addEventListener('mousedown', e => {
  isDragging = true; lastX = e.clientX; lastY = e.clientY;
  canvas.style.cursor = 'grabbing';
});
window.addEventListener('mouseup', () => { isDragging = false; canvas.style.cursor = 'grab'; });
window.addEventListener('mousemove', e => {
  if (!isDragging) return;
  targetRotY += (e.clientX - lastX) * 0.005;
  targetRotX = Math.max(0.1, Math.min(1.1, targetRotX + (e.clientY - lastY) * 0.005));
  lastX = e.clientX; lastY = e.clientY;
});

// In animation loop:
curRotX += (targetRotX - curRotX) * 0.06;
curRotY += (targetRotY - curRotY) * 0.06;
if (!isDragging) targetRotY += 0.0015; // auto-drift

const dist = 18;
camera.position.x = Math.sin(curRotY) * Math.cos(curRotX) * dist;
camera.position.y = Math.sin(curRotX) * dist;
camera.position.z = Math.cos(curRotY) * Math.cos(curRotX) * dist;
camera.lookAt(0, 0, 0);
```

**Pattern C: Scroll Parallax (camera Y follows scroll)**
Layer on top of Pattern A or B.

```javascript
let scrollY = 0;
window.addEventListener('scroll', () => { scrollY = window.scrollY; }, { passive: true });

// In animation loop:
camera.position.y = baseY - scrollY * 0.008;
camera.lookAt(0, lookAtY - scrollY * 0.004, 0);
```

---

## 2. Hero Scenes

### 2A. Orbit Diamond (Concentric Rings)

5 concentric rings with orbiting diamond octahedra + dust particles. Pointermove lerp interaction.

**Parameters:**
- `RING_COUNT` — number of rings (default: 5)
- `RING_RADII` — array of radii (default: `[3, 5, 7, 9, 11.5]`)
- `DIAMONDS_PER_RING` — octahedra per ring (default: 2)
- `DUST_COUNT` — ambient particles (default: 420)
- `ROTATION_SPEED` — base auto-rotation (default: 0.04)

```javascript
function heroOrbitDiamond(canvasId, opts = {}) {
  const {
    ringRadii = [3, 5, 7, 9, 11.5],
    diamondsPerRing = 2,
    dustCount = 420,
    rotSpeed = 0.04,
  } = opts;

  const { scene, camera, renderer, canvas } = createScene(canvasId, { fov: 38 });
  camera.position.set(0, 0, 22);

  const root = new THREE.Group();
  scene.add(root);

  // Rings (wireframe torus)
  ringRadii.forEach((r, i) => {
    const ringGeo = new THREE.RingGeometry(r - 0.01, r + 0.01, 80);
    const ringMat = new THREE.MeshBasicMaterial({
      color: i % 2 === 0 ? P.roseDim : P.dreamDim,
      side: THREE.DoubleSide, transparent: true, opacity: 0.25,
    });
    const ring = new THREE.Mesh(ringGeo, ringMat);
    ring.rotation.x = Math.PI / 2;
    root.add(ring);
  });

  // Orbiting diamonds
  const diamonds = [];
  const dGeo = new THREE.OctahedronGeometry(0.35, 0);
  ringRadii.forEach((r, ri) => {
    for (let d = 0; d < diamondsPerRing; d++) {
      const angle = (d / diamondsPerRing) * Math.PI * 2 + ri * 0.7;
      const speed = 0.3 + ri * 0.08;

      // Solid core
      const solid = new THREE.Mesh(dGeo, new THREE.MeshBasicMaterial({
        color: [P.rose, P.bone, P.dreamBright, P.roseBright, P.boneDim][ri % 5],
        transparent: true, opacity: 0.7,
      }));
      // Wireframe rim
      const wire = new THREE.Mesh(dGeo, new THREE.MeshBasicMaterial({
        color: P.roseBright, wireframe: true, transparent: true, opacity: 0.35,
      }));
      solid.add(wire);

      const group = new THREE.Group();
      group.add(solid);
      root.add(group);
      diamonds.push({ group, solid, radius: r, angle, speed, ringIdx: ri });
    }
  });

  // Dust particles
  const dustGeo = new THREE.BufferGeometry();
  const dustPos = new Float32Array(dustCount * 3);
  const dustCol = new Float32Array(dustCount * 3);
  for (let i = 0; i < dustCount; i++) {
    const r = 1 + Math.random() * 14;
    const th = Math.random() * Math.PI * 2;
    const ph = (Math.random() - 0.5) * 1.2;
    dustPos[i * 3] = Math.cos(th) * r;
    dustPos[i * 3 + 1] = Math.sin(ph) * r * 0.4;
    dustPos[i * 3 + 2] = Math.sin(th) * r;
    const c = new THREE.Color(Math.random() > 0.3 ? P.roseDim : P.dreamDim);
    dustCol[i * 3] = c.r; dustCol[i * 3 + 1] = c.g; dustCol[i * 3 + 2] = c.b;
  }
  dustGeo.setAttribute('position', new THREE.BufferAttribute(dustPos, 3));
  dustGeo.setAttribute('color', new THREE.BufferAttribute(dustCol, 3));
  root.add(new THREE.Points(dustGeo, new THREE.PointsMaterial({
    size: 0.06, vertexColors: true, transparent: true, opacity: 0.6,
    blending: THREE.AdditiveBlending, depthWrite: false,
  })));

  // Mouse lerp
  let mx = 0, my = 0, tmx = 0, tmy = 0;
  window.addEventListener('pointermove', e => {
    tmx = (e.clientX / window.innerWidth - 0.5) * 2;
    tmy = (e.clientY / window.innerHeight - 0.5) * 2;
  });

  const clock = new THREE.Clock();
  function animate() {
    requestAnimationFrame(animate);
    if (!inViewport(canvas)) return;
    const t = clock.getElapsedTime();
    mx += (tmx - mx) * 0.06;
    my += (tmy - my) * 0.06;

    root.rotation.y = mx * 0.4 + t * rotSpeed;
    root.rotation.x = my * 0.3;
    camera.position.x = mx * 1.5;
    camera.position.y = my * 1.2;

    diamonds.forEach(d => {
      const a = d.angle + t * d.speed;
      d.group.position.set(Math.cos(a) * d.radius, 0, Math.sin(a) * d.radius);
      d.solid.rotation.y = t * 1.2;
      d.solid.rotation.x = t * 0.8;
    });

    renderer.render(scene, camera);
  }
  animate();
}
```

### 2B. Two-Plane Coordination (Control + Execution Grids)

Two wireframe grid planes (control above, execution below) with a central core, instanced agents, signal octahedra, and citation arcs between planes.

**Parameters:**
- `GRID_SIZE` — plane dimensions (default: 16)
- `GRID_SEGMENTS` — wireframe divisions (default: 36)
- `AGENT_COUNT` — instanced agents on execution plane (default: 80)
- `SIGNAL_COUNT` — signal octahedra on control plane (default: 44)
- `CORE_RADIUS` — central icosahedron (default: 0.6)

```javascript
function heroTwoPlane(canvasId, opts = {}) {
  const {
    gridSize = 16, gridSegs = 36,
    agentCount = 80, signalCount = 44,
    coreRadius = 0.6,
  } = opts;

  const { scene, camera, renderer, canvas } = createScene(canvasId, {
    fov: 32, fog: { color: 0x060608, density: 0.02 }
  });
  camera.position.set(0, 6, 18);
  camera.lookAt(0, 0, 0);

  // Control plane (rose wireframe, y = +1.7)
  const controlGeo = new THREE.PlaneGeometry(gridSize, gridSize, gridSegs, gridSegs);
  const controlMat = new THREE.MeshBasicMaterial({
    color: P.roseDim, wireframe: true, transparent: true, opacity: 0.12,
  });
  const controlPlane = new THREE.Mesh(controlGeo, controlMat);
  controlPlane.rotation.x = -Math.PI / 2;
  controlPlane.position.y = 1.7;
  scene.add(controlPlane);

  // Execution plane (dream wireframe, y = -1.7)
  const execGeo = new THREE.PlaneGeometry(gridSize, gridSize, gridSegs, gridSegs);
  const execMat = new THREE.MeshBasicMaterial({
    color: P.dreamDim, wireframe: true, transparent: true, opacity: 0.12,
  });
  const execPlane = new THREE.Mesh(execGeo, execMat);
  execPlane.rotation.x = -Math.PI / 2;
  execPlane.position.y = -1.7;
  scene.add(execPlane);

  // Central core
  const coreGeo = new THREE.IcosahedronGeometry(coreRadius, 1);
  const core = new THREE.Mesh(coreGeo, new THREE.MeshBasicMaterial({
    color: P.roseBright, wireframe: true, transparent: true, opacity: 0.5,
  }));
  scene.add(core);

  // Instanced agents (execution plane)
  const agentGeo = new THREE.OctahedronGeometry(0.12, 0);
  const agentMat = new THREE.MeshBasicMaterial({ color: P.rose });
  const agents = new THREE.InstancedMesh(agentGeo, agentMat, agentCount);
  agents.instanceColor = new THREE.InstancedBufferAttribute(
    new Float32Array(agentCount * 3), 3
  );
  scene.add(agents);

  const agentData = [];
  const dummy = new THREE.Object3D();
  for (let i = 0; i < agentCount; i++) {
    const x = (Math.random() - 0.5) * gridSize * 0.8;
    const z = (Math.random() - 0.5) * gridSize * 0.8;
    agentData.push({ x, z, vx: (Math.random() - 0.5) * 0.02, vz: (Math.random() - 0.5) * 0.02 });
    const c = new THREE.Color(Math.random() > 0.15 ? P.rose : P.bone);
    agents.setColorAt(i, c);
  }

  // Signal octahedra (control plane)
  const sigGeo = new THREE.OctahedronGeometry(0.08, 0);
  const sigMat = new THREE.MeshBasicMaterial({ color: P.roseBright });
  const signals = new THREE.InstancedMesh(sigGeo, sigMat, signalCount);
  scene.add(signals);

  // Citation arcs (QuadraticBezierCurve3 from control to execution plane)
  // ... arc pool with traveling-head rendering (see Swarm arcs pattern below)

  // Drag-orbit interaction (Pattern B from Infrastructure)
  // ... attach drag-orbit handlers

  const clock = new THREE.Clock();
  function animate() {
    requestAnimationFrame(animate);
    if (!inViewport(canvas)) return;
    const t = clock.getElapsedTime();

    core.rotation.y = t * 0.3;
    core.rotation.x = t * 0.15;

    // Update agent positions
    for (let i = 0; i < agentCount; i++) {
      const a = agentData[i];
      a.x += a.vx; a.z += a.vz;
      if (Math.abs(a.x) > gridSize * 0.4) a.vx *= -1;
      if (Math.abs(a.z) > gridSize * 0.4) a.vz *= -1;
      dummy.position.set(a.x, -1.7 + 0.15, a.z);
      dummy.rotation.y = t + i;
      dummy.scale.setScalar(0.8 + Math.sin(t * 2 + i) * 0.1);
      dummy.updateMatrix();
      agents.setMatrixAt(i, dummy.matrix);
    }
    agents.instanceMatrix.needsUpdate = true;

    renderer.render(scene, camera);
  }
  animate();
}
```

### 2C. Layer Stack Cutaway (Roko + Korai Tree)

3D isometric view of stacked transparent plates (Roko layers) above a binary tree (Korai chain) with knowledge particles flowing between them.

**Parameters:**
- `LAYERS` — array of `{ name, desc, color, count, cost }` objects
- `PLATE_W, PLATE_D, PLATE_H` — plate dimensions (default: 6, 4, 0.25)
- `SPACING` — vertical gap between plates (default: 0.9)
- `TREE_LEVELS` — binary tree depth (default: 3 → 15 blocks)
- `PARTICLE_COUNT` — knowledge flow particles (default: 200)

**Interactive features:**
- Drag-orbit camera
- Raycaster hover tooltip on plates
- Layer chip panel: click/drag to remove layers → plates animate down, cost recalculates
- Press R to restore all layers

```javascript
// See rosedust-v3/v4 extraction for full implementation.
// Key patterns:
// - BoxGeometry plates with EdgesGeometry wireframe overlay
// - SphereGeometry dots on plate surfaces, pulse with sin(t * 2 + phase)
// - Recursive spawnTree() for binary tree blocks
// - Knowledge particles: smoothstep(t) * arc path, sine offset on Y
// - updateCost(): multiplier model (no gate = 8×, no VCG = 3×, etc.)
// - toggleLayer(i): removed Set, layoutStack() recomputes positions
```

### 2D. HDC Vector Field (Custom Shader Particles)

2500 particles with custom ShaderMaterial phosphor glow. "Bind" lines flash between nearby particles.

**Parameters:**
- `PARTICLE_COUNT` — visible particles (default: 2500)
- `BIND_INTERVAL` — ms between bind attempts (default: 280)
- `MAX_BINDS` — concurrent bind lines (default: 30)
- `MAX_BIND_DISTANCE` — max connection distance (default: 8)

```javascript
// Key shader:
vertexShader: `
  attribute float size;
  varying vec3 vColor;
  uniform float time;
  void main() {
    vColor = color;
    vec4 mvPosition = modelViewMatrix * vec4(position, 1.0);
    float pulse = 1.0 + 0.2 * sin(time * 0.6 + position.x * 0.3);
    gl_PointSize = size * pulse * (300.0 / -mvPosition.z);
    gl_Position = projectionMatrix * mvPosition;
  }
`,
fragmentShader: `
  varying vec3 vColor;
  void main() {
    vec2 uv = gl_PointCoord - 0.5;
    float d = length(uv);
    if (d > 0.5) discard;
    float a = smoothstep(0.5, 0.0, d);
    a = pow(a, 2.0);
    gl_FragColor = vec4(vColor, a * 0.9);
  }
`,
// Bind line search: sample 30 random particles, find nearest within MAX_BIND_DISTANCE
// Bind alpha: sin((t / life) * PI) — sine envelope fade in/out
```

---

## 3. Data Visualization Scenes

### 3A. Swarm Network (Instanced Agents + Knowledge Arcs)

Scalable agent constellation around an InsightStore column. Agents positioned via golden-angle spiral. Knowledge-transfer arcs route through the column.

**Parameters:**
- `MAX_AGENTS` — maximum instanced count (default: 1000)
- `INITIAL_AGENTS` — starting count (default: 47)
- `MAX_ARCS` — concurrent arcs (default: 40)
- `ARC_SEGMENTS` — bezier subdivisions (default: 23)
- `MOTE_COUNT` — ambient particles (default: 150)

**Slider formulas:**
```javascript
insights = Math.floor(n * 120 + Math.pow(n, 1.4) * 8);          // super-linear
gatePass = 41 + (78 - 41) * (1 - Math.exp(-n / 180));            // asymptotic → 78%
costPerEp = 0.94 * Math.exp(-n / 320) + 0.11;                    // decay → $0.11 floor
transfers = Math.floor(n * Math.log2(Math.max(n, 2)) * 2.2);     // n·log₂(n)
```

**Key patterns:**
```javascript
// Golden-angle spiral placement
const phi = Math.PI * (Math.sqrt(5) - 1);
for (let i = 0; i < MAX_AGENTS; i++) {
  const tt = i / MAX_AGENTS;
  const r = 3 + Math.pow(tt, 0.5) * 13;
  const y = (tt - 0.5) * 14;
  const theta = i * phi;
  const rxz = Math.sqrt(Math.max(0, 1 - (y/8)*(y/8))) * r * 0.5;
  positions.push({ x: Math.cos(theta) * rxz, y, z: Math.sin(theta) * rxz });
}

// Agent color variants (every 17th = bone, every 23rd = dream, rest = rose)
const variant = i % 17 === 0 ? P.bone : (i % 23 === 0 ? P.dream : P.roseBright);

// Arc traveling-head technique (bright dot slides along bezier)
const head = arc.t;
const intensity = Math.max(0, 1 - Math.abs(u - head) * 4);
// u = normalized position along arc (0..1), head = arc progress (0..1)
// Result: sharp bright spot at head, fading 25% of arc length behind

// Arc routing through InsightStore (bend midpoint toward origin)
const midX = 0, midZ = 0;
const midY = (from.y + to.y) / 2 + 2;
// Quadratic bezier: mt²·from + 2·mt·u·mid + u²·to

// Click-to-spawn/remove agent at world position
canvas.addEventListener('mouseup', e => {
  const world = canvasToWorldXZ(e); // project click to y=0 plane
  let nearest = findNearest(world, 2.5); // within 2.5 units
  if (nearest >= 0) removeAgent(nearest);
  else spawnAgent(world);
});

// Auto-scale performance guard
if (count >= 900 && fps < 30) scaleBackTo(500);
```

### 3B. Korai Prism Lattice (Chain Visualization)

Vertical column of 40 instanced glass blocks with a traveling pulse sphere.

**Parameters:**
- `N_BLOCKS` — block count (default: 40)
- `BLOCK_SIZE` — BoxGeometry dimensions (default: [1.2, 0.7, 1.2])
- `PULSE_SPEED` — curve travel speed (default: 0.4)
- `PULSE_INTERVAL` — ms between pulses (default: 400)

```javascript
// Block layout: descending column with sine-twist offset
for (let i = 0; i < N_BLOCKS; i++) {
  const y = (N_BLOCKS - 1 - i) * 0.82 - N_BLOCKS * 0.4;
  const twist = Math.sin(i * 0.2) * 0.08;
  position.set(Math.sin(i * 0.35) * 0.18, y, Math.cos(i * 0.35) * 0.18);
  rotation.set(0, twist, 0);
}

// Per-block glow: when pulse passes, glow = 1.0 → decay * 0.94 per frame
// Color shift: col.offsetHSL(0, 0, glow * 0.3)
// Edge opacity: 0.5 + glow * 0.4

// Traveling pulse: SphereGeometry(0.1) riding CatmullRomCurve3 through block spine
const curve = new THREE.CatmullRomCurve3(blockCenters.map(b => new THREE.Vector3(...)));
// Every PULSE_INTERVAL ms: reset pulseT = 0, traverse curve

// Scroll-parallax: section scroll progress → camera.position.y
const camY = 2 - sectionScrollProgress * 8;
```

### 3C. Cognitive Loop (8-Phase Orbital Ring)

8 phase nodes arranged in a ring. Active phase highlighted. Trail effect. Optional click-to-inspect with per-phase sub-mechanism visualization.

**Parameters:**
- `PHASE_COUNT` — number of phases (default: 8)
- `RING_RADIUS` — orbital radius (default: 2.4)
- `TRAIL_LENGTH` — trailing arc segments (default: per phase)
- `AUTO_ADVANCE_MS` — auto-cycle interval (default: 3500)

```javascript
// Phase node arrangement: hexagonal or circular
for (let i = 0; i < PHASE_COUNT; i++) {
  const angle = -Math.PI / 2 + (i * Math.PI * 2) / PHASE_COUNT;
  const x = cx + Math.cos(angle) * RING_RADIUS;
  const y = cy + Math.sin(angle) * RING_RADIUS;
}

// Active phase: brighter fill + glow + label
// Trail: energy dot orbiting the ring, leaving fading afterimages
// Click: zoom into sub-mechanism unique per phase:
//   OBSERVE = 4×4 probe dot grid pulsing
//   GATE = 3 horizontal tier lanes with sliding token
//   ASSEMBLE = 8 animated VCG bars
//   LLM·TOOL = 3 stacked cache layer boxes
//   REFLECT = 32×16 HDC bit grid flickering
//   CONSOLIDATE = 6-point rotating dream-wheel

// Zoom: interpolate zoomAnim 0→1: += (target - zoomAnim) * 0.12
// Escape key or click center returns to orbit
```

### 3D. MAST Failure Bars (Animated Bar Chart)

3D bar chart showing failure taxonomy with animated heights.

```javascript
// Bars: BoxGeometry(width, height, depth) per category
// Heights animate from 0 → target via lerp
// Colors from P palette based on severity
// Optional: hover to show category label + count
```

### 3E. Fractal Graph Zoom (3 Nested Levels)

Nested graph visualization with scroll-wheel zoom between levels.

**Parameters:**
- `L0_COUNT` — top-level nodes (default: 8, radius 2.4)
- `L1_PER_L0` — children per L0 (default: 5, radius 0.6)
- `L2_PER_L1` — children per L1 (default: 4, radius 0.18)

```javascript
// Scroll wheel + pointer drag control zoom target (0..2)
// zoom = lerp(zoom, target, 0.04)
// camera.position.z = 7.5 - zoom * 1.6
// As zoom increases: L0 fades, L1 becomes visible, L2 resolves
// Each level: OctahedronGeometry nodes + line connections
```

### 3F. Capability Ring Intersection (3-Ring Venn)

Three concentric/overlapping rings representing capability domains. Chips trigger pulse animations.

```javascript
// 3 rings: Space (dream, r=2.6), Graph (bone, r=1.9), Cell (rose, r=1.2)
// capSend(name): fires pulse from r=3.5 inward
// Allowed capabilities: pulse reaches center with flash
// Blocked capabilities: pulse stops at ring boundary, flashes red
```

### 3G. PPC Triangle (Predict-Publish-Correct)

3-node triangle with traveling signal pulses between nodes.

```javascript
// 3 nodes at triangle vertices
// Pulses travel along edges: predict→publish→correct→predict
// Each node pulses on receive
// Cycle time: configurable
```

### 3H. Demurrage Knowledge Field (Decay Columns)

Cylinder columns representing knowledge entries with height proportional to confidence, decaying over time.

```javascript
// CylinderGeometry columns on a grid
// Height = confidence score, decaying via half-life
// Buttons: "retrieved" (boost), "cited" (boost), "antiknow" (reduce)
// Gesell-Shannon formula: confidence *= exp(-λ * dt)
```

### 3I. Vitality Agent Lifecycle (Horizontal Track)

Single agent dot on a horizontal track with phase regions.

```javascript
// Horizontal bar with color-coded phase regions
// Agent position controlled by slider (vitality 0..1)
// Phases: THRIVING → STABLE → CONSERVATION → DECLINING → TERMINAL
// Visual: particle density, glow intensity, trail length all driven by vitality
```

---

## 4. Interactive Controls

### 4A. Prediction Error Slider (Gate Router)

Drives tier distribution (T0/T1/T2) and visualizes cost savings.

```javascript
// HTML: <input type="range" min="0" max="100" value="18" step="1">
// Distribution formulas:
const t2 = Math.min(0.95, Math.pow(pe, 1.7));
const t1 = (1 - t2) * (0.2 + pe * 0.4);
const t0 = 1 - t1 - t2;

// Blended cost per 1000 ticks:
const costs = [0.000, 0.002, 0.048]; // T0, T1, T2
const blended = (t0 * costs[0] + t1 * costs[1] + t2 * costs[2]) * 1000;
const reduction = 48.00 / Math.max(blended, 0.01);

// Advanced: draggable threshold dividers on canvas
// thresh[0] = T0/T1 boundary, thresh[1] = T1/T2 boundary
// mousedown: find nearest threshold within 6% of canvas height
// mousemove: drag threshold, bidirectionally sync with slider
```

### 4B. Agent Count Slider (Network Scaling)

Drives swarm scene + computed metrics.

```javascript
// HTML: <input type="range" min="1" max="1000" value="47" step="1">
// Metric formulas (see 3A above)
// Scene sync: updateAgentPositions(n) → agentMesh.count = n

// Vehicle spawn (Gaussian around PE):
const u = Math.random(), v = Math.random();
const gauss = Math.sqrt(-2 * Math.log(u)) * Math.cos(2 * Math.PI * v); // Box-Muller
const pev = Math.min(0.999, Math.max(0.001, pe + gauss * 0.08));
```

### 4C. Sessions Sigmoid Slider (Cost Comparison)

Shows compounding cost savings over sessions with a sigmoid curve.

```javascript
// sessions slider → Chart: sigmoid compounding
// nunchi cost: base * (1 - 0.7 * sigmoid(sessions))
// frontier cost: constant or linear growth
```

### 4D. HDC Playground (Text Input → Bit-field)

Two text inputs generating 10,240-bit binary vectors with XOR interference and Hamming similarity.

```javascript
const COLS = 128, ROWS = 80, TOTAL = 10240;

function fnv1a(str) {
  let h = 0x811c9dc5;
  for (let i = 0; i < str.length; i++) {
    h ^= str.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return h >>> 0;
}

function makePRNG(seed) {
  let s = seed || 1;
  return () => { s ^= s << 13; s ^= s >>> 17; s ^= s << 5; return (s >>> 0) / 0xffffffff; };
}

function computeField(text) {
  const field = new Uint8Array(TOTAL);
  if (!text.trim()) return field;
  const tokens = text.trim().toLowerCase().split(/\s+/);
  const sums = new Int16Array(TOTAL);
  tokens.forEach(tok => {
    const seed = fnv1a('tok:' + tok);
    const prng = makePRNG(seed);
    for (let i = 0; i < TOTAL; i++) sums[i] += prng() > 0.5 ? 1 : 0;
  });
  const thresh = Math.ceil(tokens.length / 2);
  for (let i = 0; i < TOTAL; i++) field[i] = sums[i] >= thresh ? 1 : 0;
  // 4% jitter from whole-string hash
  const sHash = fnv1a('str:' + text.trim().toLowerCase());
  const sPrng = makePRNG(sHash);
  for (let i = 0; i < TOTAL; i++) if (sPrng() < 0.04) field[i] ^= 1;
  return field;
}

// XOR interference: fi[i] = fa[i] ^ fb[i]
// Hamming distance: count differing bits
// Similarity: (1 - hamming / TOTAL) * 100
// Canvas rendering: 128×80 grid, rose = constructive (both set), dim = noise (XOR=1)
// Preset buttons: data-a="rust trait" data-b="rust impl" etc.
```

### 4E. VCG Auction Pit (Click-to-Boost Bidders)

Auto-mutating second-price auction with clickable bidder columns.

```javascript
// 8 bidders: { id, name, bid, trueValue, boost }
// Click column or mini cell: trueValue += 200, bid recalculates
// Auto-mutate every 1.2–2.4s: bid drifts toward trueValue ± noise
// Second-price line: horizontal dashed line at second-highest bid
// Flash: when secondPrice changes by >2, flash at full opacity 600ms
// Context window bar: flex-grow per bidder, smooth CSS transition
```

### 4F. Layer Chip Panel (Drag-to-Remove)

Interactive panel of draggable chips that toggle 3D scene layers.

```javascript
// Each chip: draggable, click to toggle
// Drag onto canvas = remove layer (plate animates down, cost recalculates)
// Drag back to panel = restore layer
// Cost model: removed layers apply multipliers to base cost
// R key = restore all
// Cost display updates via exponential lerp: cost += (target - cost) * 0.08
```

### 4G. Demo Triptych (Cold/Warm Scripted Replay)

Three synchronized panels (terminal + loop monitor + chain feed) driven by timestamped scripts.

```javascript
// COLD_SCRIPT / WARM_SCRIPT: array of { t, phase, line, color? }
// requestAnimationFrame advances playhead at 1× realtime
// Lines fire when script[i].t <= elapsed
// Terminal: lines append with cursor
// Monitor: phase bars update width + tick markers
// Chain: blocks mint at progress-weighted intervals
// Replay button toggles between COLD and WARM
// Auto-starts cold run on section first entering viewport
```

---

## 5. 2D Canvas Scenes

### 5A. Freeway (3-Lane Token Router)

Scrolling belt of vehicles (squares/triangles) sorted into T0/T1/T2 lanes.

```javascript
// 3 horizontal lanes with color coding: success (T0), warning (T1), rose (T2)
// Vehicles spawn every 90ms with Gaussian PE around slider value (Box-Muller)
// Lane assignment: pe < thresh[0] → T0, pe < thresh[1] → T1, else → T2
// Vehicles scroll right at 130-195 px/s + tier speed bonus
// Draggable threshold dividers: two horizontal lines on canvas
// Cost accumulator: saved += (T2_cost - actual_tier_cost) per exited vehicle
```

### 5B. Orbital Loop Console (6-Phase Hexagonal Ring)

2D canvas with 6 phase nodes on a hexagonal ring, auto-advancing active phase, with phosphor trail effect.

```javascript
// Phosphor fade: fillRect with rgba(6,6,8,0.18) instead of clearRect → motion blur
// Energy dot orbits ring, leaving trail
// Click node → zoom to sub-mechanism (unique per phase)
// Mini-dot navigation below canvas
// Auto-advance every 3.5–3.8s when section in viewport
// Stop auto-advance on hover
```

### 5C. Divergence Racetrack (Moat Compounding Chart)

Isometric track showing NUNCHI vs FRONTIER-ONLY cost curves diverging over time.

```javascript
// Scrubber slider: simT = slider.value / 1000
// Replay button: reset to t=0, auto-play
// Branching paths: NUNCHI compounds savings, FRONTIER stays linear
// SVG or canvas rendering of two diverging curves
```

### 5D. VCG Bar Chart

2D canvas bar chart with animated bid heights, second-price clearing line, and glow on click.

```javascript
// 8 columns, height proportional to bid
// Winner column: rose-bright fill + glow
// Second-price line: horizontal dashed bone line
// Click column: boost trueValue, trigger glow animation (1.2s decay)
```

---

## 6. Atmospheric & UI Components

### 6A. Atmospheric CSS Layers (Mandatory)

```css
/* 1. Grain overlay — SVG fractalNoise tile */
.grain {
  position: fixed; inset: 0;
  pointer-events: none; z-index: 9997;
  opacity: 0.07; mix-blend-mode: overlay;
  background-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='200' height='200'><filter id='n'><feTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='3' stitchTiles='stitch'/></filter><rect width='100%25' height='100%25' filter='url(%23n)' opacity='0.9'/></svg>");
}

/* 2. Scanlines — repeating gradient */
body::before {
  content: ""; position: fixed; inset: 0;
  background: repeating-linear-gradient(to bottom, transparent 0px, transparent 2px, #050507 2px, #050507 3px);
  opacity: 0.06; pointer-events: none; z-index: 9999; mix-blend-mode: multiply;
}

/* 3. Vignette — radial gradient with ambient color bleed */
body::after {
  content: ""; position: fixed; inset: 0; pointer-events: none; z-index: 9998;
  background:
    radial-gradient(ellipse at center, transparent 40%, rgba(6,6,8,0.55) 100%),
    radial-gradient(ellipse at 30% 20%, rgba(170,112,136,0.04), transparent 50%),
    radial-gradient(ellipse at 80% 80%, rgba(88,88,120,0.03), transparent 50%);
}

/* 4. CRT Flicker — on <main>, NOT on body (overlays don't flicker) */
@keyframes flicker {
  0%,100%{opacity:1} 2%{opacity:0.97} 4%{opacity:1}
  43%{opacity:1} 44%{opacity:0.93} 46%{opacity:1}
  80%{opacity:1} 82%{opacity:0.98} 84%{opacity:1}
}
main { animation: flicker 8s infinite; }
```

### 6B. Chassis Component (Instrument Panel)

```css
.chassis {
  position: relative;
  background: var(--bg-raised);
  border: 1px solid var(--border);
}
/* 4 corner screws */
.chassis > [class^="screw-"] {
  position: absolute; width: 3px; height: 3px; border-radius: 50%;
  background: var(--border); box-shadow: 0 0 0 1px rgba(24,20,32,0.9);
}
.chassis-head {
  display: flex; justify-content: space-between; align-items: center;
  padding: 6px 32px; border-bottom: 1px solid var(--border);
  font-size: 10px; letter-spacing: 0.2em; text-transform: uppercase;
  color: var(--rose-dim); background: var(--bg-mid);
}
.chassis-head .led {
  width: 6px; height: 6px; border-radius: 50%;
  background: var(--rose-bright); box-shadow: 0 0 6px var(--rose-bright);
  animation: ledPulse 1.6s ease-in-out infinite;
}
.chassis-foot {
  padding: 4px 32px; border-top: 1px solid var(--border);
  font-size: 9px; letter-spacing: 0.15em; color: var(--text-ghost);
  text-transform: uppercase; background: var(--bg-mid);
}
```

### 6C. Boot Sequence

```javascript
const bootLines = [
  { t: "ROSEDUST v4.0.0 · terminal.existential", c: "b-rose", d: 120 },
  { t: "────────────────────────────────────────", c: "", d: 40 },
  { t: "[ OK ]  crt.phosphor          engaged", c: "b-ok", d: 90 },
  // ... more lines with varying delays
  { t: "nunchi :: one integrated system", c: "b-bright", d: 300 },
];

async function runBoot() {
  let delay = 0;
  bootLines.forEach(line => {
    setTimeout(() => {
      const div = document.createElement('div');
      div.className = line.c;
      div.textContent = line.t;
      bootLog.appendChild(div);
    }, delay);
    delay += line.d;
  });
  setTimeout(() => bootEl.classList.add('done'), delay + 600);
  setTimeout(() => bootEl.remove(), delay + 1800);
}
```

### 6D. Loading Curtain (Diamond Pulse)

```css
.loading-curtain {
  position: fixed; inset: 0; z-index: 10000;
  display: flex; align-items: center; justify-content: center;
  background: var(--bg-void);
  transition: opacity 0.8s;
}
.loading-curtain.done { opacity: 0; pointer-events: none; }

.diamond {
  width: 20px; height: 20px;
  border: 1px solid var(--rose);
  transform: rotate(45deg);
  animation: diamondPulse 1.6s ease-in-out infinite;
}
@keyframes diamondPulse {
  0%,100% { transform: rotate(45deg) scale(1); opacity: 0.6; }
  50% { transform: rotate(45deg) scale(1.3); opacity: 1; box-shadow: 0 0 20px var(--rose-bright); }
}
```

### 6E. Custom Range Input (Phosphor Thumb)

```css
input[type=range] { -webkit-appearance: none; background: transparent; width: 100%; }
input[type=range]::-webkit-slider-runnable-track {
  height: 1px; background: var(--rose-deep);
}
input[type=range]::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 10px; height: 14px;
  background: var(--rose-bright);
  margin-top: -6px;
  border: 1px solid var(--rose);
  border-radius: 0; /* sharp, not rounded */
  box-shadow: 0 0 8px rgba(204,144,168,0.6);
  cursor: pointer;
}
/* Gradient track variant (for gate/freeway): */
input[type=range].gradient::-webkit-slider-runnable-track {
  height: 2px;
  background: linear-gradient(to right, var(--success), var(--warning) 60%, var(--rose-bright));
}
```

### 6F. Scroll Progress Bar

```css
.scroll-progress {
  position: fixed; top: 0; left: 0;
  height: 1px;
  background: var(--rose-bright);
  box-shadow: 0 0 6px var(--rose-bright);
  z-index: 200; width: 0%;
  transition: width 0.1s;
}
```
```javascript
window.addEventListener('scroll', () => {
  const pct = (scrollY / (document.documentElement.scrollHeight - innerHeight)) * 100;
  progressEl.style.width = pct + '%';
}, { passive: true });
```

### 6G. Scroll Reveal (Intersection Observer)

```css
.reveal { opacity: 0; transform: translateY(20px); transition: opacity 1s, transform 1s; }
.reveal.visible { opacity: 1; transform: translateY(0); }
```
```javascript
const revealer = new IntersectionObserver(entries => {
  entries.forEach(e => {
    if (e.isIntersecting) { e.target.classList.add('visible'); revealer.unobserve(e.target); }
  });
}, { threshold: 0.12 });
document.querySelectorAll('.reveal').forEach(el => revealer.observe(el));
```

### 6H. Perf HUD (FPS + Triangle Counter)

```javascript
// Fixed badge at bottom-right
let frames = 0, lastFpsTime = performance.now();
function tickPerf(dt, tris) {
  frames++;
  if (performance.now() - lastFpsTime > 500) {
    const fps = Math.round(frames / ((performance.now() - lastFpsTime) / 1000));
    fpsEl.textContent = fps;
    trisEl.textContent = tris > 999 ? (tris / 1000).toFixed(1) + 'K' : tris;
    frames = 0; lastFpsTime = performance.now();
  }
}
```

### 6I. Telemetry Sidebar (Fixed Right Panel)

```css
.telemetry {
  position: fixed; right: 0; top: 120px; bottom: 80px; width: 180px;
  border-left: 1px solid var(--border); background: rgba(6,6,8,0.85);
  backdrop-filter: blur(8px); z-index: 50;
  padding: 16px 12px; overflow-y: auto;
}
@media (max-width: 1280px) { .telemetry { display: none; } }
```
```javascript
// Values update periodically with glow flash
function updateTelemetry() {
  metrics.forEach(m => {
    const el = document.querySelector(`[data-metric="${m.id}"]`);
    el.textContent = m.compute();
    el.classList.add('flash');
    setTimeout(() => el.classList.remove('flash'), 400);
  });
}
setInterval(updateTelemetry, 2000);
```

### 6J. Definition Term Tooltips

```css
.defterm {
  text-decoration-line: underline;
  text-decoration-style: dashed;
  text-decoration-color: var(--rose-dim);
  text-underline-offset: 3px;
  cursor: help;
  position: relative;
}
.defterm:hover::after {
  content: attr(data-def);
  position: absolute; bottom: 100%; left: 0;
  background: rgba(6,6,8,0.95); border: 1px solid var(--border-active);
  border-left: 2px solid var(--rose); padding: 8px 12px;
  font-size: 12px; color: var(--text-primary);
  width: max-content; max-width: 300px;
  box-shadow: 0 4px 16px rgba(0,0,0,0.4);
  z-index: 100;
}
```
```html
<span class="defterm" data-def="Hyperdimensional Computing uses 10,240-bit vectors...">HDC</span>
```
