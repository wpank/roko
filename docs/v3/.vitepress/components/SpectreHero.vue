<template>
  <div
    ref="containerRef"
    class="spectre-hero"
    @mousemove="onMouseMove"
    @mouseleave="onMouseLeave"
  >
    <canvas ref="canvasRef" class="spectre-hero__canvas" />
    <!-- CSS fallback when WebGL is unavailable -->
    <div v-if="useFallback" class="spectre-hero__fallback">
      <div class="spectre-hero__fallback-orb">
        <div class="spectre-hero__fallback-eye spectre-hero__fallback-eye--left" />
        <div class="spectre-hero__fallback-eye spectre-hero__fallback-eye--right" />
      </div>
      <div class="spectre-hero__fallback-particles">
        <span
          v-for="i in 12"
          :key="i"
          class="spectre-hero__fallback-particle"
          :style="fallbackParticleStyle(i)"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
/**
 * SpectreHero -- Three.js WebGL Spectre creature for the docs landing page.
 *
 * A procedural Orb-archetype Spectre rendered as an instanced dot-cloud
 * with spring connections, emissive eyes that blink and track the mouse,
 * wisdom-sparkle particles, and UnrealBloom + vignette post-processing.
 *
 * ROSEDUST palette:
 *   body:       #c77d8f (rose)
 *   glow:       #ffc0d0 (rose-glow)
 *   eyeFrost:   #e5e7eb (frost)
 *   eyeIris:    #dc9bb4 (rose-bright)
 *   particle1:  #a78bfa (violet)
 *   particle2:  #c77d8f (rose)
 *   springDim:  #8b5e6b (rose-dim)
 *   bg:         #0a0a0f (void-black)
 *
 * Breathing: 0.7 Hz, depth 0.06 (+-6%), asymmetric inhale/exhale.
 * Triangle budget: < 50,000.
 */
import { ref, onMounted, onBeforeUnmount } from 'vue'

// ── ROSEDUST hex constants for Three.js ──
const CLR = {
  body:      0xc77d8f,
  glow:      0xffc0d0,
  eyeFrost:  0xe5e7eb,
  eyeIris:   0xdc9bb4,
  particle1: 0xa78bfa,
  particle2: 0xc77d8f,
  springDim: 0x8b5e6b,
  bg:        0x0a0a0f,
}

const containerRef = ref<HTMLElement | null>(null)
const canvasRef    = ref<HTMLCanvasElement | null>(null)
const useFallback  = ref(false)

// ── Mouse state (normalised -1..1) ──
const mouse = { x: 0, y: 0, active: false }

function onMouseMove(e: MouseEvent) {
  if (!containerRef.value) return
  const r = containerRef.value.getBoundingClientRect()
  mouse.x = ((e.clientX - r.left) / r.width) * 2 - 1
  mouse.y = -((e.clientY - r.top) / r.height) * 2 + 1
  mouse.active = true
}

function onMouseLeave() {
  mouse.active = false
}

/** CSS fallback particle positioning */
function fallbackParticleStyle(i: number) {
  return {
    '--angle': `${(i / 12) * 360}deg`,
    '--delay': `${(i * 0.4).toFixed(1)}s`,
    '--dist': `${80 + (i % 3) * 30}px`,
  }
}

// Deterministic pseudo-random from a seed (used for body perturbation)
function seeded(seed: number): number {
  const x = Math.sin(seed * 127.1 + 311.7) * 43758.5453
  return x - Math.floor(x)
}

let animId = 0
let teardown: (() => void) | null = null

onMounted(async () => {
  if (!canvasRef.value || !containerRef.value) return

  // ── WebGL probe ──
  const probe = canvasRef.value.getContext('webgl2') || canvasRef.value.getContext('webgl')
  if (!probe) {
    useFallback.value = true
    return
  }
  // Release the test context; Three.js creates its own.
  const loseExt = probe.getExtension('WEBGL_lose_context')
  if (loseExt) loseExt.loseContext()

  // ── Dynamic imports (client-side only) ──
  const THREE = await import('three')
  const { EffectComposer }  = await import('three/examples/jsm/postprocessing/EffectComposer.js')
  const { RenderPass }      = await import('three/examples/jsm/postprocessing/RenderPass.js')
  const { UnrealBloomPass } = await import('three/examples/jsm/postprocessing/UnrealBloomPass.js')
  const { ShaderPass }      = await import('three/examples/jsm/postprocessing/ShaderPass.js')

  // ── Scene ──
  const scene = new THREE.Scene()
  scene.background = new THREE.Color(CLR.bg)

  const w = containerRef.value.clientWidth
  const h = containerRef.value.clientHeight

  const camera = new THREE.PerspectiveCamera(45, w / h, 0.1, 100)
  camera.position.set(0, 0, 6)
  camera.lookAt(0, 0, 0)

  const renderer = new THREE.WebGLRenderer({
    canvas: canvasRef.value,
    antialias: true,
    alpha: false,
  })
  renderer.setSize(w, h)
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2))
  renderer.toneMapping = THREE.ACESFilmicToneMapping
  renderer.toneMappingExposure = 1.2

  // ───────────────────────────────────────────────
  //  Body: Fibonacci-sphere dot cloud (Orb archetype)
  // ───────────────────────────────────────────────
  const BODY_N = 320
  const bodyPos: THREE.Vector3[]     = []
  const bodyBase: THREE.Vector3[]    = []

  const golden = Math.PI * (3 - Math.sqrt(5))
  for (let i = 0; i < BODY_N; i++) {
    const y  = 1 - (i / (BODY_N - 1)) * 2
    const rY = Math.sqrt(1 - y * y)
    const th = golden * i
    const p  = 0.08 // perturbation amplitude

    const px = Math.cos(th) * rY + (seeded(i * 3    ) - 0.5) * p
    const py = y                  + (seeded(i * 3 + 1) - 0.5) * p
    const pz = Math.sin(th) * rY + (seeded(i * 3 + 2) - 0.5) * p

    const v = new THREE.Vector3(px, py, pz)
    bodyPos.push(v)
    bodyBase.push(v.clone())
  }

  // Instanced mesh for body dots (6-seg sphere = 96 tris each, 320 x 96 = 30 720 tris)
  const dotGeo = new THREE.SphereGeometry(0.028, 6, 4)
  const dotMat = new THREE.MeshStandardMaterial({
    color: CLR.body,
    emissive: CLR.body,
    emissiveIntensity: 0.4,
    roughness: 0.5,
    metalness: 0.2,
  })
  const bodyMesh = new THREE.InstancedMesh(dotGeo, dotMat, BODY_N)
  scene.add(bodyMesh)

  const tmp = new THREE.Object3D()
  function syncBodyInstances() {
    for (let i = 0; i < BODY_N; i++) {
      tmp.position.copy(bodyPos[i])
      tmp.updateMatrix()
      bodyMesh.setMatrixAt(i, tmp.matrix)
    }
    bodyMesh.instanceMatrix.needsUpdate = true
  }
  syncBodyInstances()

  // ───────────────────────────────────────────────
  //  Spring connections between nearby points
  // ───────────────────────────────────────────────
  const SPRING_DIST = 0.35
  const allPairs: [number, number][] = []
  for (let i = 0; i < BODY_N; i++) {
    for (let j = i + 1; j < BODY_N; j++) {
      if (bodyBase[i].distanceTo(bodyBase[j]) < SPRING_DIST) {
        allPairs.push([i, j])
      }
    }
  }
  // Deterministic cap at 600 for GPU budget
  const MAX_SPRINGS = 600
  const springs = allPairs.length > MAX_SPRINGS
    ? allPairs.sort((a, b) => seeded(a[0] + a[1]) - seeded(b[0] + b[1])).slice(0, MAX_SPRINGS)
    : allPairs

  const springBuf = new Float32Array(springs.length * 6)
  const springGeo = new THREE.BufferGeometry()
  springGeo.setAttribute('position', new THREE.BufferAttribute(springBuf, 3))
  const springMat = new THREE.LineBasicMaterial({
    color: CLR.springDim,
    transparent: true,
    opacity: 0.25,
  })
  const springLines = new THREE.LineSegments(springGeo, springMat)
  scene.add(springLines)

  function syncSprings() {
    for (let s = 0; s < springs.length; s++) {
      const [a, b] = springs[s]
      const pa = bodyPos[a], pb = bodyPos[b]
      const o = s * 6
      springBuf[o    ] = pa.x; springBuf[o + 1] = pa.y; springBuf[o + 2] = pa.z
      springBuf[o + 3] = pb.x; springBuf[o + 4] = pb.y; springBuf[o + 5] = pb.z
    }
    springGeo.attributes.position.needsUpdate = true
  }

  // ───────────────────────────────────────────────
  //  Eyes (two emissive spheres + irises)
  // ───────────────────────────────────────────────
  const eyeGeo = new THREE.SphereGeometry(0.09, 16, 12)
  const eyeMatL = new THREE.MeshStandardMaterial({
    color: CLR.eyeFrost,
    emissive: CLR.eyeFrost,
    emissiveIntensity: 0.9,
    roughness: 0.1,
    metalness: 0.0,
  })
  const eyeMatR = eyeMatL.clone()

  const eyeL = new THREE.Mesh(eyeGeo, eyeMatL)
  const eyeR = new THREE.Mesh(eyeGeo, eyeMatR)

  const irisGeo = new THREE.SphereGeometry(0.045, 12, 8)
  const irisMatL = new THREE.MeshStandardMaterial({
    color: CLR.eyeIris, emissive: CLR.eyeIris, emissiveIntensity: 0.6,
    roughness: 0.2, metalness: 0.0,
  })
  const irisMatR = irisMatL.clone()
  const irisL = new THREE.Mesh(irisGeo, irisMatL)
  const irisR = new THREE.Mesh(irisGeo, irisMatR)

  const EYE_X = 0.28, EYE_Y = 0.18, EYE_Z = 0.85
  eyeL.position.set(-EYE_X, EYE_Y, EYE_Z)
  eyeR.position.set( EYE_X, EYE_Y, EYE_Z)
  irisL.position.set(0, 0, 0.05)
  irisR.position.set(0, 0, 0.05)
  eyeL.add(irisL)
  eyeR.add(irisR)

  // Eyelids (discs that scale-Y to simulate blinking)
  const lidGeo = new THREE.CircleGeometry(0.1, 16)
  const lidMat = new THREE.MeshBasicMaterial({ color: CLR.bg, side: THREE.DoubleSide })
  const lidL = new THREE.Mesh(lidGeo, lidMat)
  const lidR = new THREE.Mesh(lidGeo, lidMat.clone())
  lidL.position.set(0, 0, 0.06)
  lidR.position.set(0, 0, 0.06)
  lidL.scale.set(1, 0, 1)
  lidR.scale.set(1, 0, 1)
  eyeL.add(lidL)
  eyeR.add(lidR)

  scene.add(eyeL)
  scene.add(eyeR)

  // ───────────────────────────────────────────────
  //  Wisdom-sparkle particle system
  // ───────────────────────────────────────────────
  const P_N = 80
  const pBuf = new Float32Array(P_N * 3)
  interface PVel { x: number; y: number; z: number; life: number; maxLife: number }
  const pVel: PVel[] = []

  function resetP(i: number) {
    const a = Math.random() * Math.PI * 2
    const r = 1.0 + Math.random() * 0.6
    pBuf[i * 3    ] = Math.cos(a) * r
    pBuf[i * 3 + 1] = (Math.random() - 0.5) * 1.5
    pBuf[i * 3 + 2] = Math.sin(a) * r
    pVel[i] = {
      x: (Math.random() - 0.5) * 0.003,
      y: 0.005 + Math.random() * 0.008,
      z: (Math.random() - 0.5) * 0.003,
      life: 0,
      maxLife: 3 + Math.random() * 4,
    }
  }
  for (let i = 0; i < P_N; i++) resetP(i)

  const pGeo = new THREE.BufferGeometry()
  pGeo.setAttribute('position', new THREE.BufferAttribute(pBuf, 3))
  const pMat = new THREE.PointsMaterial({
    color: CLR.particle1,
    size: 0.04,
    transparent: true,
    opacity: 0.6,
    blending: THREE.AdditiveBlending,
    depthWrite: false,
  })
  scene.add(new THREE.Points(pGeo, pMat))

  // ───────────────────────────────────────────────
  //  Lighting
  // ───────────────────────────────────────────────
  scene.add(new THREE.AmbientLight(0x1a0f1a, 0.3))

  const keyL = new THREE.PointLight(CLR.glow, 1.5, 12)
  keyL.position.set(2, 3, 4)
  scene.add(keyL)

  const fillL = new THREE.PointLight(CLR.particle1, 0.6, 10)
  fillL.position.set(-3, -1, 2)
  scene.add(fillL)

  const rimL = new THREE.PointLight(CLR.glow, 0.8, 8)
  rimL.position.set(0, -2, -3)
  scene.add(rimL)

  // ───────────────────────────────────────────────
  //  Post-processing: Bloom + Vignette
  // ───────────────────────────────────────────────
  const composer = new EffectComposer(renderer)
  composer.addPass(new RenderPass(scene, camera))

  const bloom = new UnrealBloomPass(
    new THREE.Vector2(w, h),
    0.8,  // strength
    0.4,  // radius
    0.3,  // threshold
  )
  composer.addPass(bloom)

  const vignetteShader = {
    uniforms: {
      tDiffuse:  { value: null as THREE.Texture | null },
      uOffset:   { value: 0.95 },
      uDarkness: { value: 1.4 },
    },
    vertexShader: /* glsl */ `
      varying vec2 vUv;
      void main() {
        vUv = uv;
        gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
      }
    `,
    fragmentShader: /* glsl */ `
      uniform sampler2D tDiffuse;
      uniform float uOffset;
      uniform float uDarkness;
      varying vec2 vUv;
      void main() {
        vec4 c = texture2D(tDiffuse, vUv);
        vec2 uv = (vUv - 0.5) * vec2(uOffset);
        float vig = clamp(1.0 - dot(uv, uv), 0.0, 1.0);
        c.rgb *= mix(1.0 - uDarkness, 1.0, vig);
        gl_FragColor = c;
      }
    `,
  }
  composer.addPass(new ShaderPass(vignetteShader))

  // ───────────────────────────────────────────────
  //  Animation state
  // ───────────────────────────────────────────────
  const BREATH_HZ    = 0.7
  const BREATH_DEPTH = 0.06 // +-6 %
  let breathPhase    = 0

  let blinkTimer = 3 + Math.random() * 4
  let blinkLeft  = 0          // seconds remaining in current blink
  const BLINK_DUR = 0.15

  const sm = { x: 0, y: 0 }  // smoothed mouse
  const SMOOTH = 0.06

  let prevT = performance.now()
  const c1 = new THREE.Color(CLR.particle1)
  const c2 = new THREE.Color(CLR.particle2)

  // ── Render loop ──────────────────────────────
  function tick(now: number) {
    animId = requestAnimationFrame(tick)
    const dt = Math.min((now - prevT) / 1000, 0.05)
    prevT = now

    // Smooth mouse
    if (mouse.active) {
      sm.x += (mouse.x - sm.x) * SMOOTH
      sm.y += (mouse.y - sm.y) * SMOOTH
    } else {
      sm.x *= 0.97
      sm.y *= 0.97
    }

    // ── Breathing (asymmetric: fast inhale, slow exhale) ──
    breathPhase += dt * BREATH_HZ * Math.PI * 2
    const raw = Math.sin(breathPhase)
    const shaped = raw >= 0
      ? Math.pow(raw, 0.7)       // inhale: shorter (compressed)
      : -Math.pow(-raw, 1.4)     // exhale: longer (stretched)

    const hoverMul = mouse.active ? 1.3 : 1.0
    const scale = 1 + shaped * BREATH_DEPTH * hoverMul

    // ── Body points ──
    const t = now * 0.001
    const swX = sm.x * 0.12
    const swY = sm.y * 0.08
    for (let i = 0; i < BODY_N; i++) {
      const b = bodyBase[i]
      const ph = t * 0.5 + i * 0.03
      bodyPos[i].set(
        b.x * scale + Math.sin(ph * 1.1) * 0.015 + swX,
        b.y * scale + Math.cos(ph * 0.9) * 0.015 + swY,
        b.z * scale + Math.sin(ph * 0.7 + 1.0) * 0.01,
      )
    }
    syncBodyInstances()
    syncSprings()

    // ── Eyes: iris tracks mouse ──
    const iX = sm.x * 0.04
    const iY = sm.y * 0.03
    irisL.position.set(iX, iY, 0.05)
    irisR.position.set(iX, iY, 0.05)
    eyeL.position.set(-EYE_X + swX, EYE_Y + swY, EYE_Z)
    eyeR.position.set( EYE_X + swX, EYE_Y + swY, EYE_Z)

    // ── Blinking ──
    blinkTimer -= dt
    if (blinkTimer <= 0 && blinkLeft === 0) {
      blinkLeft  = BLINK_DUR
      blinkTimer = 2.5 + Math.random() * 5
    }
    if (blinkLeft > 0) {
      blinkLeft -= dt
      const prog = 1 - blinkLeft / BLINK_DUR
      const lid  = prog < 0.3 ? prog / 0.3 : 1 - (prog - 0.3) / 0.7
      const sv   = Math.max(0, lid)
      lidL.scale.set(1, sv, 1)
      lidR.scale.set(1, sv, 1)
      if (blinkLeft <= 0) {
        blinkLeft = 0
        lidL.scale.set(1, 0, 1)
        lidR.scale.set(1, 0, 1)
      }
    }

    // Eye glow pulse
    const ep = 0.7 + Math.sin(t * 1.2) * 0.15 + (mouse.active ? 0.15 : 0)
    eyeMatL.emissiveIntensity = ep
    eyeMatR.emissiveIntensity = ep

    // ── Particles ──
    for (let i = 0; i < P_N; i++) {
      const v = pVel[i]
      v.life += dt
      if (v.life >= v.maxLife) { resetP(i); continue }
      pBuf[i * 3    ] += v.x
      pBuf[i * 3 + 1] += v.y
      pBuf[i * 3 + 2] += v.z
    }
    pGeo.attributes.position.needsUpdate = true

    // Shimmer particle colour between violet and rose
    const mix = (Math.sin(t * 0.3) + 1) * 0.5
    pMat.color.copy(c1).lerp(c2, mix)
    pMat.opacity = 0.5

    // ── Dynamic post-processing ──
    bloom.strength = mouse.active ? 1.0 : 0.8
    dotMat.emissiveIntensity = 0.35 + (mouse.active ? 0.15 : 0) + Math.sin(t * 0.8) * 0.05

    composer.render()
  }

  animId = requestAnimationFrame(tick)

  // ── Resize ──
  function onResize() {
    if (!containerRef.value) return
    const rw = containerRef.value.clientWidth
    const rh = containerRef.value.clientHeight
    camera.aspect = rw / rh
    camera.updateProjectionMatrix()
    renderer.setSize(rw, rh)
    composer.setSize(rw, rh)
    bloom.resolution.set(rw, rh)
  }

  const ro = new ResizeObserver(onResize)
  ro.observe(containerRef.value)

  // ── Teardown closure ──
  teardown = () => {
    ro.disconnect()
    cancelAnimationFrame(animId)
    renderer.dispose()
    composer.dispose()
    ;[dotGeo, springGeo, eyeGeo, irisGeo, lidGeo, pGeo].forEach(g => g.dispose())
    ;[dotMat, springMat, eyeMatL, eyeMatR, irisMatL, irisMatR, lidMat, pMat].forEach(m => m.dispose())
  }
})

onBeforeUnmount(() => {
  cancelAnimationFrame(animId)
  if (teardown) teardown()
})
</script>

<style scoped>
.spectre-hero {
  position: relative;
  width: 100%;
  height: 400px;
  overflow: hidden;
  background: #0a0a0f;
  border-radius: 12px;
  margin-bottom: 2rem;
  cursor: crosshair;
}

.spectre-hero__canvas {
  display: block;
  width: 100%;
  height: 100%;
}

/* ── CSS animated fallback ── */

.spectre-hero__fallback {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #0a0a0f;
}

.spectre-hero__fallback-orb {
  position: relative;
  width: 120px;
  height: 120px;
  border-radius: 50%;
  background: radial-gradient(
    circle at 40% 40%,
    #d4778c 0%,
    #c77d8f 40%,
    #8b5e6b 80%,
    transparent 100%
  );
  box-shadow:
    0 0 40px rgba(199, 125, 143, 0.5),
    0 0 80px rgba(199, 125, 143, 0.2),
    inset 0 0 20px rgba(255, 192, 208, 0.15);
  animation: fb-breathe 1.43s ease-in-out infinite;
}

.spectre-hero__fallback-eye {
  position: absolute;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: radial-gradient(
    circle,
    #e5e7eb 30%,
    #dc9bb4 70%,
    transparent 100%
  );
  box-shadow: 0 0 8px rgba(229, 231, 235, 0.8);
  top: 38%;
  animation: fb-blink 4s ease-in-out infinite;
}

.spectre-hero__fallback-eye--left  { left: 32%; }
.spectre-hero__fallback-eye--right { right: 32%; animation-delay: 0.1s; }

.spectre-hero__fallback-particles {
  position: absolute;
  inset: 0;
}

.spectre-hero__fallback-particle {
  position: absolute;
  left: 50%;
  top: 50%;
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: #a78bfa;
  opacity: 0;
  animation: fb-particle 5s ease-in-out infinite;
  animation-delay: var(--delay);
  transform: rotate(var(--angle)) translateX(var(--dist));
  box-shadow: 0 0 6px rgba(167, 139, 250, 0.6);
}

@keyframes fb-breathe {
  0%, 100% { transform: scale(1); }
  40%      { transform: scale(1.06); }
}

@keyframes fb-blink {
  0%, 92%, 100% { transform: scaleY(1); }
  95%           { transform: scaleY(0.05); }
}

@keyframes fb-particle {
  0%   { opacity: 0; transform: rotate(var(--angle)) translateX(var(--dist)) translateY(0); }
  15%  { opacity: 0.7; }
  85%  { opacity: 0.3; }
  100% { opacity: 0; transform: rotate(var(--angle)) translateX(var(--dist)) translateY(-60px); }
}

/* ── Responsive ── */

@media (max-width: 768px) {
  .spectre-hero {
    height: 280px;
  }
}

@media (max-width: 480px) {
  .spectre-hero {
    height: 220px;
  }

  .spectre-hero__fallback-orb {
    width: 80px;
    height: 80px;
  }
}
</style>
