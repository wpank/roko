<template>
  <div class="pipeline-flow-3d" ref="rootRef">
    <canvas ref="canvasRef" class="pipeline-canvas" />
    <div ref="labelLayer" class="pf3d-label-layer"></div>

    <!-- Controls overlay -->
    <div class="pf3d-controls">
      <button @click="togglePlay" class="pf3d-btn" :title="playing ? 'Pause' : 'Play'">
        {{ playing ? 'Pause' : 'Play' }}
      </button>
      <label class="pf3d-speed-control">
        <span class="pf3d-speed-label">Speed</span>
        <input
          type="range"
          min="0.2"
          max="3"
          step="0.1"
          v-model.number="speedFactor"
        />
      </label>
    </div>

    <!-- Stage detail panel -->
    <transition name="pf3d-panel">
      <div
        v-if="selectedStage !== null"
        class="pf3d-detail-overlay"
        @click.self="selectedStage = null"
      >
        <div class="pf3d-detail-card">
          <button class="pf3d-detail-close" @click="selectedStage = null">x</button>
          <h4 :style="{ color: STAGES[selectedStage].color }">
            {{ STAGES[selectedStage].label }}
          </h4>
          <p class="pf3d-detail-crate">{{ STAGES[selectedStage].crate }}</p>
          <p class="pf3d-detail-desc">{{ STAGES[selectedStage].description }}</p>
        </div>
      </div>
    </transition>
  </div>
</template>

<script setup>
/**
 * PipelineFlow3D -- dramatic 3D visualization of the 8-stage cognitive pipeline.
 *
 * Uses Three.js with:
 *  - 8 hexagonal-prism platforms arranged in a curved arc
 *  - ~20 glowing Signal particles flowing along Catmull-Rom splines
 *  - Branching at verify (pass/fail with loopback)
 *  - T0 reflex short-circuit arc (~20% skip query -> react)
 *  - UnrealBloomPass for particle glow
 *  - Per-stage speed modulation and color transitions
 *  - Click-to-inspect, idle auto-orbit, play/pause, speed slider
 *
 * ROSEDUST semantic palette:
 *   query/score  -> sapphire
 *   route/compose -> violet
 *   act          -> rose
 *   verify       -> jade
 *   write        -> amber
 *   react        -> rose-bright
 */
import { ref, onMounted, onBeforeUnmount, shallowRef } from 'vue'

// ── ROSEDUST-derived semantic colors ──────────────────────────────
const COLORS = {
  sapphire:   '#4A7BF7',
  violet:     '#7C5CDB',
  rose:       '#B97894',
  jade:       '#5DAE8B',
  amber:      '#D4A843',
  roseBright: '#DC9BB4',
  failRed:    '#C36E55',
  reflexDim:  '#7A6878',
}

// ── Stage definitions ────────────────────────────────────────────
const STAGES = [
  {
    label: 'Query',
    crate: 'roko-core',
    color: COLORS.sapphire,
    description:
      'A prompt or task enters the system as a Signal. The query is parsed, classified by domain, and tagged with metadata (session, priority, constraints).',
    speedMul: 1.0,
  },
  {
    label: 'Score',
    crate: 'roko-learn',
    color: COLORS.sapphire,
    description:
      'Existing knowledge is scored for relevance. The cascade router selects a model tier. Playbook matches and episodic memory are consulted.',
    speedMul: 1.0,
  },
  {
    label: 'Route',
    crate: 'roko-gateway',
    color: COLORS.violet,
    description:
      'The inference gateway routes through its nine-stage pipeline: caching, cost checks, backpressure, and provider selection with fallback.',
    speedMul: 1.4,
  },
  {
    label: 'Compose',
    crate: 'roko-compose',
    color: COLORS.violet,
    description:
      'The 9-layer SystemPromptBuilder assembles the prompt: role template, safety preamble, domain context, knowledge injection, tool descriptions, and task-specific enrichment.',
    speedMul: 1.4,
  },
  {
    label: 'Act',
    crate: 'roko-agent',
    color: COLORS.rose,
    description:
      'The selected provider executes the composed prompt. The tool loop runs: the agent calls tools, receives results, and iterates until completion or budget exhaustion.',
    speedMul: 1.0,
  },
  {
    label: 'Verify',
    crate: 'roko-gate',
    color: COLORS.jade,
    description:
      'The 7-rung gate pipeline validates the output: compile check, test suite, clippy, diff review, oracle evaluation, and safety screening. Adaptive thresholds apply.',
    speedMul: 0.6,
  },
  {
    label: 'Write',
    crate: 'roko-fs',
    color: COLORS.amber,
    description:
      'Verified results are persisted: signal appended to JSONL log, episodes recorded, learning telemetry flushed, and knowledge tiers updated.',
    speedMul: 1.0,
  },
  {
    label: 'React',
    crate: 'roko-conductor',
    color: COLORS.roseBright,
    description:
      'Reactive watchers fire: the conductor evaluates whether to trigger replanning, update routing weights, run dream consolidation, or publish events to the bus.',
    speedMul: 1.0,
  },
]

// ── Tuning constants ─────────────────────────────────────────────
const PARTICLE_COUNT = 22
const TRAIL_COUNT = 4
const T0_RATIO = 0.20    // fraction of particles that take the T0 reflex shortcut
const FAIL_RATIO = 0.15  // fraction of main-path particles that fail at verify
const ARC_RADIUS = 8.0
const ARC_HALF_ANGLE = Math.PI * 0.55
const PLATFORM_RADIUS = 0.55
const PLATFORM_HEIGHT = 0.35
const COMPONENT_HEIGHT = 500
const IDLE_MS = 4000     // start auto-orbit after this many ms without interaction

// ── Reactive state ───────────────────────────────────────────────
const rootRef = ref(null)
const canvasRef = ref(null)
const labelLayer = ref(null)
const playing = ref(true)
const speedFactor = ref(1.0)
const selectedStage = ref(null)

// Non-reactive Three.js handle (cleaned up in onBeforeUnmount)
const state = shallowRef(null)

function togglePlay() {
  playing.value = !playing.value
}

// ── Lifecycle ────────────────────────────────────────────────────
onMounted(async () => {
  // SSR guard
  if (typeof window === 'undefined') return

  const container = rootRef.value
  const el = canvasRef.value
  const labelRoot = labelLayer.value
  if (!container || !el || !labelRoot) return

  // Dynamic imports -- Three.js is heavy; keep it out of the SSR bundle.
  const THREE = await import('three')
  const { EffectComposer } = await import('three/examples/jsm/postprocessing/EffectComposer.js')
  const { RenderPass } = await import('three/examples/jsm/postprocessing/RenderPass.js')
  const { UnrealBloomPass } = await import('three/examples/jsm/postprocessing/UnrealBloomPass.js')
  const { OutputPass } = await import('three/examples/jsm/postprocessing/OutputPass.js')

  const width = container.clientWidth
  const height = COMPONENT_HEIGHT

  // ────── Renderer ──────
  const renderer = new THREE.WebGLRenderer({ canvas: el, antialias: true, alpha: true })
  renderer.setSize(width, height)
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2))
  renderer.setClearColor(0x000000, 0)
  renderer.toneMapping = THREE.ACESFilmicToneMapping
  renderer.toneMappingExposure = 1.2

  // ────── Scene ──────
  const scene = new THREE.Scene()

  // ────── Camera ──────
  const camera = new THREE.PerspectiveCamera(45, width / height, 0.1, 100)
  camera.position.set(0, 6, 14)
  camera.lookAt(0, 0, 0)

  // ────── Lighting ──────
  scene.add(new THREE.AmbientLight(0x404050, 0.6))
  const sun = new THREE.DirectionalLight(0xffffff, 0.8)
  sun.position.set(5, 10, 7)
  scene.add(sun)

  // ────── Compute platform positions along a curved arc ──────
  const platformPositions = []
  for (let i = 0; i < 8; i++) {
    const t = i / 7
    const angle = -ARC_HALF_ANGLE + t * 2 * ARC_HALF_ANGLE
    const x = ARC_RADIUS * Math.sin(angle)
    const z = ARC_RADIUS * Math.cos(angle) - ARC_RADIUS + 2
    // gentle upward bow in the middle
    const y = -0.4 + 0.3 * Math.sin(t * Math.PI)
    platformPositions.push(new THREE.Vector3(x, y, z))
  }

  // ────── Build platforms (hexagonal prisms) ──────
  const platformMeshes = []
  const platformGlows = []
  const hitTargets = []

  for (let i = 0; i < 8; i++) {
    const color = new THREE.Color(STAGES[i].color)
    const pos = platformPositions[i]

    // Hexagonal prism: CylinderGeometry with 6 radial segments
    const geo = new THREE.CylinderGeometry(
      PLATFORM_RADIUS, PLATFORM_RADIUS * 1.05, PLATFORM_HEIGHT, 6, 1
    )
    const mat = new THREE.MeshStandardMaterial({
      color,
      emissive: color,
      emissiveIntensity: 0.3,
      metalness: 0.4,
      roughness: 0.5,
      transparent: true,
      opacity: 0.92,
    })
    const mesh = new THREE.Mesh(geo, mat)
    mesh.position.copy(pos)
    mesh.userData.stageIndex = i
    scene.add(mesh)
    platformMeshes.push(mesh)

    // Glow ring beneath
    const glowGeo = new THREE.RingGeometry(PLATFORM_RADIUS * 0.8, PLATFORM_RADIUS * 1.3, 32)
    const glowMat = new THREE.MeshBasicMaterial({
      color,
      transparent: true,
      opacity: 0.15,
      side: THREE.DoubleSide,
    })
    const glowMesh = new THREE.Mesh(glowGeo, glowMat)
    glowMesh.rotation.x = -Math.PI / 2
    glowMesh.position.set(pos.x, pos.y - PLATFORM_HEIGHT / 2 - 0.01, pos.z)
    scene.add(glowMesh)
    platformGlows.push(glowMesh)

    // Invisible click target (oversized cylinder for forgiving hit detection)
    const hitGeo = new THREE.CylinderGeometry(
      PLATFORM_RADIUS * 1.5, PLATFORM_RADIUS * 1.5, PLATFORM_HEIGHT * 2, 8
    )
    const hitMesh = new THREE.Mesh(hitGeo, new THREE.MeshBasicMaterial({ visible: false }))
    hitMesh.position.copy(pos)
    hitMesh.userData.stageIndex = i
    scene.add(hitMesh)
    hitTargets.push(hitMesh)
  }

  // ────── DOM labels (positioned via projection each frame) ──────
  const labelEls = []
  for (let i = 0; i < 8; i++) {
    const div = document.createElement('div')
    div.className = 'pf3d-stage-label'
    div.textContent = STAGES[i].label
    div.style.color = STAGES[i].color
    div.style.textShadow = '0 0 8px ' + STAGES[i].color + '40'
    labelRoot.appendChild(div)
    labelEls.push(div)
  }

  // ────── Spline paths ──────
  // Elevated copies of platform positions (particles ride above the platforms)
  const elevatedPositions = platformPositions.map(
    p => p.clone().setY(p.y + PLATFORM_HEIGHT / 2 + 0.15)
  )

  // Main pipeline: 8 stages
  const mainSpline = new THREE.CatmullRomCurve3(elevatedPositions, false, 'centripetal', 0.5)

  // T0 reflex shortcut: query -> react, arcing above
  const t0Start = elevatedPositions[0].clone()
  const t0End = elevatedPositions[7].clone()
  const t0Mid = new THREE.Vector3(
    (t0Start.x + t0End.x) / 2,
    Math.max(t0Start.y, t0End.y) + 3.2,
    (t0Start.z + t0End.z) / 2 - 1.5,
  )
  const t0Spline = new THREE.CatmullRomCurve3(
    [t0Start, t0Mid, t0End], false, 'centripetal', 0.5
  )

  // Verify-fail loopback: verify -> compose
  const failStart = elevatedPositions[5].clone()
  const failEnd = elevatedPositions[3].clone()
  const failMid = new THREE.Vector3(
    (failStart.x + failEnd.x) / 2,
    Math.max(failStart.y, failEnd.y) + 2.0,
    (failStart.z + failEnd.z) / 2 - 1.8,
  )
  const failSpline = new THREE.CatmullRomCurve3(
    [failStart, failMid, failEnd], false, 'centripetal', 0.5
  )

  // Render path tubes
  function addTube(spline, hex, opacity) {
    const geo = new THREE.TubeGeometry(spline, 64, 0.02, 4, false)
    const mat = new THREE.MeshBasicMaterial({
      color: new THREE.Color(hex),
      transparent: true,
      opacity,
    })
    scene.add(new THREE.Mesh(geo, mat))
  }
  addTube(mainSpline, '#555566', 0.3)
  addTube(t0Spline, COLORS.reflexDim, 0.15)
  addTube(failSpline, COLORS.failRed, 0.15)

  // ────── Particle helpers ──────
  function colorAtProgress(t, pathType) {
    if (pathType === 't0') return new THREE.Color(COLORS.reflexDim)
    if (pathType === 'fail') {
      const c = new THREE.Color()
      c.lerpColors(new THREE.Color(COLORS.jade), new THREE.Color(COLORS.failRed), t)
      return c
    }
    // Main pipeline: blend between stage colors
    const idx = Math.min(Math.floor(t * 8), 7)
    const next = Math.min(idx + 1, 7)
    const frac = t * 8 - idx
    const c = new THREE.Color()
    c.lerpColors(new THREE.Color(STAGES[idx].color), new THREE.Color(STAGES[next].color), frac)
    return c
  }

  function speedAtProgress(t, pathType) {
    if (pathType === 't0') return 2.5
    if (pathType === 'fail') return 1.2
    return STAGES[Math.min(Math.floor(t * 8), 7)].speedMul
  }

  // ────── Create one particle (orb + trail ghosts) ──────
  function spawnParticle() {
    const isT0 = Math.random() < T0_RATIO
    const pathType = isT0 ? 't0' : 'main'
    const spline = isT0 ? t0Spline : mainSpline
    const baseOpacity = isT0 ? 0.4 : 0.9

    const orbGeo = new THREE.SphereGeometry(0.08, 12, 12)
    const orbMat = new THREE.MeshBasicMaterial({
      color: 0xffffff,
      transparent: true,
      opacity: baseOpacity,
    })
    const orb = new THREE.Mesh(orbGeo, orbMat)
    scene.add(orb)

    const trails = []
    for (let t = 0; t < TRAIL_COUNT; t++) {
      const tGeo = new THREE.SphereGeometry(0.06 - t * 0.01, 8, 8)
      const tMat = new THREE.MeshBasicMaterial({
        color: 0xffffff,
        transparent: true,
        opacity: baseOpacity * 0.6 * (1 - (t + 1) / (TRAIL_COUNT + 1)),
      })
      const tMesh = new THREE.Mesh(tGeo, tMat)
      scene.add(tMesh)
      trails.push(tMesh)
    }

    return {
      orb,
      trails,
      progress: 0,
      pathType,
      spline,
      willFail: pathType === 'main' && Math.random() < FAIL_RATIO,
      failTriggered: false,
    }
  }

  // Initialise particles staggered across the path
  const particles = []
  for (let i = 0; i < PARTICLE_COUNT; i++) {
    const p = spawnParticle()
    p.progress = Math.random()
    particles.push(p)
  }

  // ────── Post-processing: bloom ──────
  const composer = new EffectComposer(renderer)
  composer.addPass(new RenderPass(scene, camera))
  const bloom = new UnrealBloomPass(
    new THREE.Vector2(width, height),
    0.8,  // strength
    0.4,  // radius
    0.85, // threshold
  )
  composer.addPass(bloom)
  composer.addPass(new OutputPass())

  // ────── Raycaster for click-to-inspect ──────
  const raycaster = new THREE.Raycaster()
  const pointer = new THREE.Vector2()

  function onCanvasClick(event) {
    const rect = el.getBoundingClientRect()
    pointer.x = ((event.clientX - rect.left) / rect.width) * 2 - 1
    pointer.y = -((event.clientY - rect.top) / rect.height) * 2 + 1
    raycaster.setFromCamera(pointer, camera)
    const hits = raycaster.intersectObjects(hitTargets)
    if (hits.length > 0) {
      const idx = hits[0].object.userData.stageIndex
      selectedStage.value = selectedStage.value === idx ? null : idx
    }
  }
  el.addEventListener('click', onCanvasClick)

  // ────── Idle orbit tracking ──────
  let lastInteraction = Date.now()
  let orbitAngle = 0
  function resetIdleTimer() { lastInteraction = Date.now() }
  el.addEventListener('mousemove', resetIdleTimer)
  el.addEventListener('touchstart', resetIdleTimer, { passive: true })

  // ────── Animation loop ──────
  const clock = new THREE.Clock()
  // Wrap in an object so the cleanup closure always reads the latest ID
  const anim = { id: null }

  function tick() {
    anim.id = requestAnimationFrame(tick)
    const delta = clock.getDelta()
    const elapsed = clock.getElapsedTime()

    // Always project labels even when paused
    projectLabels()

    if (!playing.value) {
      composer.render()
      return
    }

    const spd = speedFactor.value

    // ── Update particles ──
    for (const p of particles) {
      // Shift trail positions backward before advancing the head
      for (let t = TRAIL_COUNT - 1; t > 0; t--) {
        p.trails[t].position.copy(p.trails[t - 1].position)
      }
      if (p.trails.length > 0) {
        p.trails[0].position.copy(p.orb.position)
      }

      // Advance
      const localSpeed = speedAtProgress(p.progress, p.pathType)
      p.progress += delta * 0.12 * spd * localSpeed

      // Verify-stage fail branch
      if (
        p.pathType === 'main' &&
        p.willFail &&
        !p.failTriggered &&
        p.progress >= 5.0 / 8.0
      ) {
        p.failTriggered = true
        p.pathType = 'fail'
        p.spline = failSpline
        p.progress = 0
        p.orb.material.color.set(COLORS.failRed)
        for (const tm of p.trails) tm.material.color.set(COLORS.failRed)
      }

      // Recycle when reaching end
      if (p.progress >= 1.0) {
        if (p.pathType === 'fail') {
          // Re-enter main pipeline at compose (stage 3)
          p.pathType = 'main'
          p.spline = mainSpline
          p.progress = 3.0 / 8.0
          p.willFail = false
          p.failTriggered = false
          p.orb.material.opacity = 0.9
          p.orb.material.color.set(STAGES[3].color)
          for (const tm of p.trails) tm.material.opacity = 0.3
        } else {
          // Recycle from the beginning with a new random path type
          const isT0 = Math.random() < T0_RATIO
          p.pathType = isT0 ? 't0' : 'main'
          p.spline = isT0 ? t0Spline : mainSpline
          p.progress = 0
          p.willFail = p.pathType === 'main' && Math.random() < FAIL_RATIO
          p.failTriggered = false
          p.orb.material.opacity = isT0 ? 0.4 : 0.9
          for (const tm of p.trails) tm.material.opacity = (isT0 ? 0.15 : 0.3)
        }
      }

      // Position on spline
      const clampedT = Math.max(0, Math.min(1, p.progress))
      p.orb.position.copy(p.spline.getPointAt(clampedT))

      // Color transition
      const col = colorAtProgress(clampedT, p.pathType)
      p.orb.material.color.copy(col)
      for (const tm of p.trails) tm.material.color.copy(col)

      // Pulsing scale
      p.orb.scale.setScalar(1.0 + 0.15 * Math.sin(elapsed * 4 + p.progress * 20))
    }

    // ── Animate platforms ──
    for (let i = 0; i < 8; i++) {
      // Glow breathe
      platformGlows[i].material.opacity = 0.12 + 0.06 * Math.sin(elapsed * 1.5 + i * 0.8)
      // Subtle bob
      platformMeshes[i].position.y =
        platformPositions[i].y + 0.05 * Math.sin(elapsed * 0.8 + i * 0.5)
      // Slow spin
      platformMeshes[i].rotation.y = elapsed * 0.1 + i * Math.PI / 8
    }

    // ── Idle auto-orbit ──
    if (Date.now() - lastInteraction > IDLE_MS) {
      orbitAngle += delta * 0.15
      camera.position.x = 14 * Math.sin(orbitAngle) * 0.3
      camera.position.z = 14 * Math.cos(orbitAngle * 0.5) * 0.7 + 6
      camera.position.y = 6 + Math.sin(orbitAngle * 0.3) * 0.5
      camera.lookAt(0, 0, 0)
    }

    composer.render()
  }

  function projectLabels() {
    const sz = new THREE.Vector2()
    renderer.getSize(sz)
    for (let i = 0; i < 8; i++) {
      const world = platformPositions[i].clone()
      world.y += PLATFORM_HEIGHT / 2 + 0.45
      const ndc = world.project(camera)
      const px = (ndc.x * 0.5 + 0.5) * sz.x
      const py = (-ndc.y * 0.5 + 0.5) * sz.y
      const lbl = labelEls[i]
      if (ndc.z > 1) {
        lbl.style.display = 'none'
      } else {
        lbl.style.display = ''
        lbl.style.left = px + 'px'
        lbl.style.top = py + 'px'
      }
    }
  }

  // ────── Resize handler ──────
  function onResize() {
    const w = container.clientWidth
    const h = COMPONENT_HEIGHT
    renderer.setSize(w, h)
    camera.aspect = w / h
    camera.updateProjectionMatrix()
    composer.setSize(w, h)
    bloom.resolution.set(w, h)
  }
  window.addEventListener('resize', onResize)

  tick()

  // Store handles for cleanup
  state.value = {
    anim,
    renderer,
    scene,
    composer,
    el,
    onResize,
    onCanvasClick,
    resetIdleTimer,
    labelEls,
  }
})

onBeforeUnmount(() => {
  const s = state.value
  if (!s) return

  cancelAnimationFrame(s.anim.id)
  window.removeEventListener('resize', s.onResize)
  s.el.removeEventListener('click', s.onCanvasClick)
  s.el.removeEventListener('mousemove', s.resetIdleTimer)
  s.el.removeEventListener('touchstart', s.resetIdleTimer)

  // Dispose all Three.js geometry and materials
  s.scene.traverse((obj) => {
    if (obj.geometry) obj.geometry.dispose()
    if (obj.material) {
      if (Array.isArray(obj.material)) {
        obj.material.forEach(m => m.dispose())
      } else {
        obj.material.dispose()
      }
    }
  })
  s.renderer.dispose()
  s.composer.dispose()

  for (const lbl of s.labelEls) lbl.remove()
})
</script>

<style scoped>
.pipeline-flow-3d {
  position: relative;
  width: 100%;
  height: 500px;
  overflow: hidden;
  border-radius: 8px;
  background: transparent;
  margin: 1.5rem 0;
}

.pipeline-canvas {
  display: block;
  width: 100%;
  height: 100%;
}

.pf3d-label-layer {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
  overflow: hidden;
}

/* ── Controls ── */
.pf3d-controls {
  position: absolute;
  bottom: 12px;
  left: 12px;
  display: flex;
  align-items: center;
  gap: 12px;
  z-index: 10;
}

.pf3d-btn {
  padding: 5px 16px;
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: 4px;
  background: rgba(14, 12, 18, 0.7);
  color: #dcc6c8;
  cursor: pointer;
  font-size: 13px;
  font-weight: 500;
  backdrop-filter: blur(6px);
  transition: background 0.2s, border-color 0.2s;
}

.pf3d-btn:hover {
  background: rgba(185, 120, 148, 0.25);
  border-color: rgba(185, 120, 148, 0.4);
}

.pf3d-speed-control {
  display: flex;
  align-items: center;
  gap: 6px;
}

.pf3d-speed-label {
  color: rgba(165, 142, 158, 0.7);
  font-size: 12px;
}

.pf3d-speed-control input[type='range'] {
  width: 80px;
  accent-color: #B97894;
  cursor: pointer;
}

/* ── Stage detail panel ── */
.pf3d-detail-overlay {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 20;
  background: rgba(0, 0, 0, 0.4);
  backdrop-filter: blur(4px);
}

.pf3d-detail-card {
  position: relative;
  max-width: 380px;
  padding: 20px 24px;
  background: rgba(14, 12, 18, 0.92);
  border: 1px solid rgba(185, 120, 148, 0.25);
  border-radius: 10px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
}

.pf3d-detail-card h4 {
  margin: 0 0 4px;
  font-size: 18px;
  font-weight: 700;
}

.pf3d-detail-crate {
  margin: 0 0 10px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, monospace;
  font-size: 12px;
  color: rgba(165, 142, 158, 0.7);
}

.pf3d-detail-desc {
  margin: 0;
  font-size: 13px;
  line-height: 1.65;
  color: #D7C6D0;
}

.pf3d-detail-close {
  position: absolute;
  top: 8px;
  right: 12px;
  background: none;
  border: none;
  color: rgba(165, 142, 158, 0.5);
  cursor: pointer;
  font-size: 16px;
  font-weight: 600;
  line-height: 1;
  padding: 4px;
}

.pf3d-detail-close:hover {
  color: #DC9BB4;
}

/* ── Panel transition ── */
.pf3d-panel-enter-active,
.pf3d-panel-leave-active {
  transition: opacity 0.2s ease;
}
.pf3d-panel-enter-from,
.pf3d-panel-leave-to {
  opacity: 0;
}

/* ── Responsive ── */
@media (max-width: 768px) {
  .pipeline-flow-3d {
    height: 360px;
  }
}
</style>

<style>
/* Unscoped: labels are created dynamically via DOM API */
.pf3d-stage-label {
  position: absolute;
  transform: translate(-50%, -100%);
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.5px;
  text-transform: uppercase;
  pointer-events: none;
  white-space: nowrap;
  user-select: none;
}
</style>
