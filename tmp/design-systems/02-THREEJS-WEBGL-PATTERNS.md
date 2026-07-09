# Three.js / WebGL Implementation Patterns

Use this document when building interactive Three.js scenes, particle systems, and WebGL backgrounds for ROSEDUST-themed sites. All patterns are extracted from working implementations.

---

## Setup Pattern

```javascript
import * as THREE from "three";
// Use importmap: { "imports": { "three": "https://unpkg.com/three@0.160.0/build/three.module.js" } }

const can = document.querySelector("canvas");
const renderer = new THREE.WebGLRenderer({ canvas: can, antialias: true, alpha: true });
renderer.setPixelRatio(Math.min(devicePixelRatio, 2));

const scene = new THREE.Scene();
const cam = new THREE.PerspectiveCamera(56, 1, 0.1, 500);
cam.position.set(0, 1, 18);

const root = new THREE.Group();
scene.add(root);
```

Always use `alpha: true` for compositing over dark CSS backgrounds. Cap pixel ratio at 2 for performance.

---

## Scene 1: Particle Swarm (Hero Background)

Instanced mesh of small octahedra that swarm, orbit, and transition between chaos and coordination states.

```javascript
const N = 220;  // particle count — 200-300 is the sweet spot

// Use InstancedMesh for performance — one draw call for all particles
const inst = new THREE.InstancedMesh(
  new THREE.OctahedronGeometry(0.18, 0),
  new THREE.MeshBasicMaterial({ color: 0xaa7088, transparent: true, opacity: 0.85 }),
  N
);
inst.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
inst.instanceColor = new THREE.InstancedBufferAttribute(new Float32Array(N * 3), 3);
inst.instanceColor.setUsage(THREE.DynamicDrawUsage);
root.add(inst);
```

### Per-Particle Data Structure
```javascript
const agents = [];
for (let i = 0; i < N; i++) {
  const ang = Math.random() * Math.PI * 2;
  const rad = 3 + Math.random() * 8;
  agents.push({
    x: Math.cos(ang) * rad,
    y: (Math.random() - 0.5) * 6,
    z: Math.sin(ang) * rad,
    vx: 0, vy: 0, vz: 0,
    tx: 0, ty: 0, tz: 0,        // target position
    phase: Math.random() * Math.PI * 2,
    speed: 0.3 + Math.random() * 0.7,
    orbitR: rad,
    orbitY: (Math.random() - 0.5) * 4,
    sz: 0.6 + Math.random() * 0.8,
    col: [0.67, 0.44, 0.53]      // rose in RGB
  });
}
```

### Two States: Chaos vs Coordination

**Chaos (swarm):** Particles orbit independently with slight randomization
```javascript
// In animation loop:
a.tx = Math.cos(t * a.speed + a.phase) * a.orbitR;
a.ty = Math.sin(t * a.speed * 0.7 + a.phase) * 2 + a.orbitY;
a.tz = Math.sin(t * a.speed + a.phase) * a.orbitR;
```

**Coordination (organized):** Particles converge into structured formations — planes, rings, grid
```javascript
// Control plane formation: agents orbit a central octahedron in organized layers
const layer = Math.floor(i / layerSize);
const angleInLayer = (i % layerSize) / layerSize * Math.PI * 2;
a.tx = Math.cos(angleInLayer + t * 0.2) * (2 + layer * 1.5);
a.ty = layer * 0.8 - 1.5;
a.tz = Math.sin(angleInLayer + t * 0.2) * (2 + layer * 1.5);
```

### Smooth Transitions Between States
```javascript
const lerp = 0.03;  // Exponential approach — 3% per frame
a.x += (a.tx - a.x) * lerp;
a.y += (a.ty - a.y) * lerp;
a.z += (a.tz - a.z) * lerp;
```

### Color Per Particle
```javascript
// ROSEDUST palette in RGB floats:
const ROSE     = [0.67, 0.44, 0.53];   // #aa7088
const ROSE_GL  = [0.80, 0.56, 0.66];   // #cc90a8
const ROSE_DIM = [0.48, 0.31, 0.38];   // #7a5060
const BONE     = [0.85, 0.78, 0.63];   // #d8c8a0
const DREAM    = [0.58, 0.58, 0.70];   // #9494b4

// Set color per instance:
inst.setColorAt(i, new THREE.Color(c[0], c[1], c[2]));
inst.instanceColor.needsUpdate = true;
```

### Composing Each Instance Transform
```javascript
const tmpMat = new THREE.Matrix4();
const tmpQ   = new THREE.Quaternion();
const tmpV   = new THREE.Vector3();

// Per particle in animation loop:
tmpV.set(a.x, a.y, a.z);
tmpQ.setFromAxisAngle(new THREE.Vector3(0, 1, 0), t * 0.5 + a.phase);
tmpMat.compose(tmpV, tmpQ, new THREE.Vector3(a.sz, a.sz, a.sz));
inst.setMatrixAt(i, tmpMat);

inst.instanceMatrix.needsUpdate = true;
```

---

## Scene 2: Armillary Sphere (Orrery)

A central octahedral core with orbiting rings, nodes, and cosmic dust.

### Core Assembly
```javascript
// Solid inner core
const core = new THREE.Mesh(
  new THREE.OctahedronGeometry(0.7, 1),
  new THREE.MeshBasicMaterial({ color: 0xdca5bd, transparent: true, opacity: 0.85 })
);

// Wireframe rim (slightly larger)
const coreRim = new THREE.Mesh(
  new THREE.OctahedronGeometry(0.78, 0),
  new THREE.MeshBasicMaterial({ color: 0xdca5bd, transparent: true, opacity: 0.5, wireframe: true })
);

// Outer halo (much larger, very transparent)
const coreHalo = new THREE.Mesh(
  new THREE.OctahedronGeometry(1.6, 0),
  new THREE.MeshBasicMaterial({ color: 0xdca5bd, transparent: true, opacity: 0.14, wireframe: true })
);
```

### Orbit Rings
```javascript
const ringR = 5.5;

// Primary ring
const ring = new THREE.Mesh(
  new THREE.RingGeometry(ringR - 0.04, ringR + 0.04, 256),
  new THREE.MeshBasicMaterial({ color: 0x7a5060, transparent: true, opacity: 0.55, side: THREE.DoubleSide })
);
ring.rotation.x = Math.PI / 2;

// Inner ring
const ring2 = new THREE.Mesh(
  new THREE.RingGeometry(ringR - 0.7, ringR - 0.55, 256),
  new THREE.MeshBasicMaterial({ color: 0xaa7088, transparent: true, opacity: 0.16, side: THREE.DoubleSide })
);
ring2.rotation.x = Math.PI / 2;

// Outer halo ring
const ring3 = new THREE.Mesh(
  new THREE.RingGeometry(ringR + 0.15, ringR + 0.85, 256),
  new THREE.MeshBasicMaterial({ color: 0xaa7088, transparent: true, opacity: 0.10, side: THREE.DoubleSide })
);
ring3.rotation.x = Math.PI / 2;
```

### Orbiting Nodes
```javascript
const nodeCount = 9;
const nodes = [];
for (let i = 0; i < nodeCount; i++) {
  const g = new THREE.Group();
  const angle = (i / nodeCount) * Math.PI * 2;
  g.position.set(Math.cos(angle) * ringR, 0, Math.sin(angle) * ringR);

  // Solid octahedron
  const oct = new THREE.Mesh(
    new THREE.OctahedronGeometry(0.42, 1),
    new THREE.MeshBasicMaterial({ color: 0x7a5060, transparent: true, opacity: 0.92 })
  );
  g.add(oct);

  // Wireframe overlay
  const wf = new THREE.Mesh(
    new THREE.OctahedronGeometry(0.46, 0),
    new THREE.MeshBasicMaterial({ color: 0xdca5bd, transparent: true, opacity: 0.4, wireframe: true })
  );
  g.add(wf);

  // Halo
  const halo = new THREE.Mesh(
    new THREE.OctahedronGeometry(0.95, 0),
    new THREE.MeshBasicMaterial({ color: 0x7a5060, transparent: true, opacity: 0.16, wireframe: true })
  );
  g.add(halo);

  // Equatorial disc
  const disc = new THREE.Mesh(
    new THREE.RingGeometry(0.55, 0.66, 32),
    new THREE.MeshBasicMaterial({ color: 0xaa7088, transparent: true, opacity: 0.42, side: THREE.DoubleSide })
  );
  disc.rotation.x = Math.PI / 2;
  g.add(disc);

  root.add(g);
  nodes.push({ group: g, angle, speed: 0.12 + Math.random() * 0.08 });
}
```

### Cosmic Dust (Points)
```javascript
const dustN = 160;
const dustGeom = new THREE.BufferGeometry();
const dpos = new Float32Array(dustN * 3);
for (let i = 0; i < dustN; i++) {
  const r = 2 + Math.random() * 12;
  const th = Math.random() * Math.PI * 2;
  const ph = (Math.random() - 0.5) * Math.PI;
  dpos[i*3]   = Math.cos(th) * Math.cos(ph) * r;
  dpos[i*3+1] = Math.sin(ph) * r * 0.4;
  dpos[i*3+2] = Math.sin(th) * Math.cos(ph) * r;
}
dustGeom.setAttribute("position", new THREE.BufferAttribute(dpos, 3));

const dust = new THREE.Points(dustGeom, new THREE.PointsMaterial({
  color: 0xdca5bd,
  size: 0.05,
  transparent: true,
  opacity: 0.5,
  blending: THREE.AdditiveBlending,
  depthWrite: false
}));
root.add(dust);
```

### Spokes (Radial Lines)
```javascript
for (let i = 0; i < 12; i++) {
  const ang = (i / 12) * Math.PI * 2;
  const g = new THREE.BufferGeometry();
  g.setAttribute("position", new THREE.Float32BufferAttribute(
    [0, 0, 0, Math.cos(ang) * ringR, 0, Math.sin(ang) * ringR], 3
  ));
  const ln = new THREE.Line(g, new THREE.LineBasicMaterial({
    color: 0x7a5060, transparent: true, opacity: 0.18
  }));
  root.add(ln);
}
```

---

## Scene 3: Wireframe Grid (Architectural / Control Plane)

Concentric grid planes with wireframe octahedra at intersections.

```javascript
const gridSize = 5;
for (let r = 0; r < gridSize; r++) {
  for (let c = 0; c < gridSize; c++) {
    const g = new THREE.OctahedronGeometry(0.55, 0);
    const m = new THREE.MeshBasicMaterial({
      color: (r + c) % 2 === 0 ? 0xdca5bd : 0xd8c8a0,  // alternating rose-glow / bone
      transparent: true,
      opacity: 0.45,
      wireframe: true
    });
    const p = new THREE.Mesh(g, m);
    p.position.set(c * 2 - gridSize, 0, r * 2 - gridSize);
    root.add(p);
  }
}
```

---

## Animation Loop Pattern

```javascript
function animate(time) {
  requestAnimationFrame(animate);
  const t = time * 0.001;

  // Resize handling
  const { clientWidth: w, clientHeight: h } = can.parentElement;
  if (can.width !== w * devicePixelRatio || can.height !== h * devicePixelRatio) {
    renderer.setSize(w, h);
    cam.aspect = w / h;
    cam.updateProjectionMatrix();
  }

  // Slow root rotation (gives depth without user interaction)
  root.rotation.y = t * 0.08;
  root.rotation.x = Math.sin(t * 0.15) * 0.12;

  // Update particles...
  // Update orbiting nodes...

  renderer.render(scene, cam);
}
animate(0);
```

---

## WebGL Background Primitives (Canvas 2D / Raw WebGL)

### Particle Field (No Three.js dependency)
```javascript
// Raw canvas particle system — lighter weight than Three.js
const ctx = canvas.getContext('2d');
const particles = Array.from({ length: 80 }, () => ({
  x: Math.random() * w,
  y: Math.random() * h,
  vx: (Math.random() - 0.5) * 0.3,
  vy: (Math.random() - 0.5) * 0.3,
  r: 1 + Math.random() * 2,
  color: ['#aa7088', '#c8b890', '#7a7a98'][Math.floor(Math.random() * 3)]
}));

function draw() {
  ctx.clearRect(0, 0, w, h);
  particles.forEach(p => {
    // Draw dot
    ctx.beginPath();
    ctx.arc(p.x, p.y, p.r, 0, Math.PI * 2);
    ctx.fillStyle = p.color;
    ctx.globalAlpha = 0.6;
    ctx.fill();

    // Draw connections to nearby particles
    particles.forEach(q => {
      const dx = p.x - q.x, dy = p.y - q.y;
      const dist = Math.sqrt(dx*dx + dy*dy);
      if (dist < 120 && dist > 0) {
        ctx.beginPath();
        ctx.moveTo(p.x, p.y);
        ctx.lineTo(q.x, q.y);
        ctx.strokeStyle = p.color;
        ctx.globalAlpha = 0.08 * (1 - dist / 120);
        ctx.stroke();
      }
    });

    // Update position
    p.x += p.vx; p.y += p.vy;
    if (p.x < 0 || p.x > w) p.vx *= -1;
    if (p.y < 0 || p.y > h) p.vy *= -1;
  });
  requestAnimationFrame(draw);
}
```

### Ambient Page Particles (Fixed position, very subtle)
```css
.ambient { position: fixed; inset: 0; pointer-events: none; z-index: 1; opacity: 0.8; }
.ambient canvas { position: absolute; inset: 0; width: 100%; height: 100%; }
```

### Noise Background (WebGL Fragment Shader)
```glsl
uniform float u_time;
uniform float u_scale;
uniform vec3 u_colorA;  // --bg-void
uniform vec3 u_colorB;  // --rose-deep

// Simplex noise function here...

void main() {
  vec2 uv = gl_FragCoord.xy / u_resolution;
  float n = snoise(vec3(uv * u_scale, u_time * 0.3));
  vec3 color = mix(u_colorA, u_colorB, n * 0.5 + 0.5);
  gl_FragColor = vec4(color, 1.0);
}
```

---

## Canvas Wrapper Pattern

All Three.js scenes are wrapped in a container with consistent styling:

```html
<div class="cwrap" style="height: 560px;">
  <canvas></canvas>
  <!-- Optional HUD overlays -->
  <div class="hud tl">
    <span>AGENTS</span>
    <span class="v r">220</span>
  </div>
  <div class="hud br" style="text-align: right;">
    <span>STATUS</span>
    <span class="v b">COORDINATED</span>
  </div>
</div>
```

```css
.cwrap {
  position: relative; width: 100%;
  background: radial-gradient(ellipse at center, rgba(58,32,48,0.18) 0%, #040406 65%);
  overflow: hidden;
}
.cwrap::after {
  content: ""; position: absolute; inset: 0; pointer-events: none;
  background: radial-gradient(ellipse at center, transparent 60%, rgba(0,0,0,0.5) 100%);
  z-index: 2;
}
.cwrap canvas { position: absolute; inset: 0; width: 100% !important; height: 100% !important; }
```

### HUD Overlays
```css
.hud {
  position: absolute; font-family: var(--mono); font-size: 10px;
  letter-spacing: 0.24em; text-transform: uppercase;
  color: var(--text-dim); pointer-events: none;
}
.hud .v { font-size: 14px; color: var(--text-soft); margin-top: 5px; }
.hud .v.b { color: var(--bone-bright); }
.hud .v.r { color: var(--rose-glow); }
.hud .v.disp { font-family: var(--display); font-style: italic; font-size: 18px; }
.hud.tl { top: 18px; left: 22px; }
.hud.tr { top: 18px; right: 22px; text-align: right; }
.hud.bl { bottom: 18px; left: 22px; }
.hud.br { bottom: 18px; right: 22px; text-align: right; }
```

---

## Performance Budget

```
Three.js vertices:              ≤ 2,000
Canvas particles:               ≤ 60 (for 2D canvas)
Instanced mesh count:           ≤ 300
Points (dust/ambient):          ≤ 200
Animation frame budget:         ≤ 4ms per frame
Pixel ratio cap:                2
```

Always use `InstancedMesh` instead of individual meshes. Use `AdditiveBlending` for dust/glow particles. Set `depthWrite: false` on transparent point materials.

---

## ROSEDUST Three.js Color Constants

```javascript
const ROSE       = 0xaa7088;
const ROSE_BRIGHT= 0xcc90a8;
const ROSE_GLOW  = 0xdca5bd;
const ROSE_DIM   = 0x7a5060;
const ROSE_DEEP  = 0x3a2030;
const BONE       = 0xc8b890;
const BONE_BR    = 0xd8c8a0;
const DREAM      = 0x7a7a98;
const SUCCESS    = 0x7a8a78;
const WARNING    = 0xc89a68;
const BG_VOID    = 0x060608;
const BG_DEEPER  = 0x040406;
```

---

## Key Techniques

1. **Octahedra over spheres**: Geometric, crystalline, distinctive
2. **Wireframe + solid layering**: Solid core + wireframe rim + transparent halo = depth
3. **Additive blending for dust**: `blending: THREE.AdditiveBlending` gives ethereal glow
4. **Ring geometry for orbit paths**: Flat rings at various opacities suggest structure
5. **InstancedMesh for particles**: One draw call, hundreds of objects
6. **Exponential approach for transitions**: `x += (target - x) * 0.03` — smooth, organic
7. **Slow root rotation**: `root.rotation.y = t * 0.08` — constant gentle motion
8. **Radial gradient container**: CSS gradient behind canvas creates depth
9. **HUD overlays**: Absolute-positioned mono text gives data-viz feel
10. **State toggles**: UI buttons transition between scene configurations smoothly
