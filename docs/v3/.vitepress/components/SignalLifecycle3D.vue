<template>
  <div ref="rootEl" class="signal-lifecycle-3d">
    <canvas ref="canvasEl" />

    <!-- Tier info overlay -->
    <div v-if="selectedTier" class="tier-info" :style="tierInfoStyle">
      <div class="tier-info__name">{{ selectedTier.name }}</div>
      <div class="tier-info__desc">{{ selectedTier.desc }}</div>
      <div class="tier-info__retention">{{ selectedTier.retention }}</div>
    </div>

    <!-- Signal hover tooltip -->
    <div v-if="hoveredSignal" class="signal-tooltip" :style="tooltipStyle">
      <div class="signal-tooltip__tier">{{ hoveredSignal.tierName }}</div>
      <div class="signal-tooltip__score">Score: {{ hoveredSignal.score.toFixed(2) }}</div>
      <div class="signal-tooltip__age">Age: {{ hoveredSignal.ageSec.toFixed(1) }}s</div>
      <div class="signal-tooltip__state">{{ hoveredSignal.state }}</div>
    </div>

    <!-- Time controls -->
    <div class="lifecycle-controls">
      <button
        class="ctrl-btn"
        :class="{ active: timeScale === 0.25 }"
        @click="timeScale = 0.25"
        title="Slow"
      >0.25x</button>
      <button
        class="ctrl-btn"
        :class="{ active: timeScale === 1 }"
        @click="timeScale = 1"
        title="Normal"
      >1x</button>
      <button
        class="ctrl-btn"
        :class="{ active: timeScale === 3 }"
        @click="timeScale = 3"
        title="Fast"
      >3x</button>
      <span class="ctrl-label">Signals: {{ signalCount }}</span>
    </div>
  </div>
</template>

<script setup>
import { ref, reactive, onMounted, onUnmounted, shallowRef } from 'vue'

// ── ROSEDUST palette ──
const COLORS = {
  ghost:    0x4a3545,  // Transient: dim purple-grey
  mist:     0x8b5e6b,  // Working: dusty rose
  frost:    0xc77d8f,  // Consolidated: rosedust primary
  roseGlow: 0xe0a9b3,  // Persistent: warm rose glow
  gold:     0xd4a843,  // Golden highlight for Persistent ring
  bg:       0x0a0a0f,  // Void black
  ring:     0x2a2535,  // Divider tone
  particle: 0xc77d8f,  // Trail particles
  decay:    0x6b3a4a,  // Fading signal
  text:     0xe5e7eb,  // Text color
}

const TIER_DEFS = [
  {
    name: 'Transient',
    y: 0,
    radius: 2.8,
    color: COLORS.ghost,
    opacity: 0.35,
    signalSize: 0.12,
    desc: 'May be pruned aggressively (minutes). Default tier for new Signals.',
    retention: 'Retention: minutes',
  },
  {
    name: 'Working',
    y: 2.5,
    radius: 3.4,
    color: COLORS.mist,
    opacity: 0.55,
    signalSize: 0.18,
    desc: 'Retained during active task scope. Requires effective score above threshold.',
    retention: 'Retention: task scope',
  },
  {
    name: 'Consolidated',
    y: 5.0,
    radius: 4.0,
    color: COLORS.frost,
    opacity: 0.75,
    signalSize: 0.24,
    desc: 'Survives across sessions. Feeds learning subsystem.',
    retention: 'Retention: cross-session',
  },
  {
    name: 'Persistent',
    y: 7.5,
    radius: 4.6,
    color: COLORS.roseGlow,
    opacity: 1.0,
    signalSize: 0.3,
    desc: 'Permanent archive. Never auto-pruned. The golden tier.',
    retention: 'Retention: permanent',
  },
]

const canvasEl = ref(null)
const rootEl = ref(null)
const selectedTier = ref(null)
const hoveredSignal = ref(null)
const tierInfoStyle = reactive({ left: '0px', top: '0px' })
const tooltipStyle = reactive({ left: '0px', top: '0px' })
const timeScale = ref(1)
const signalCount = ref(0)

// Three.js objects kept as shallow refs to avoid Vue reactivity overhead
const threeState = shallowRef(null)

let animationId = null
let isVisible = true
let prefersReducedMotion = false

onMounted(async () => {
  const motionQuery = window.matchMedia('(prefers-reduced-motion: reduce)')
  prefersReducedMotion = motionQuery.matches
  if (prefersReducedMotion) return

  const THREE = await import('three')

  const canvas = canvasEl.value
  const root = rootEl.value
  if (!canvas || !root) return

  const width = root.clientWidth
  const height = 400

  // ── Renderer ──
  const renderer = new THREE.WebGLRenderer({
    canvas,
    alpha: true,
    antialias: true,
  })
  renderer.setSize(width, height)
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2))
  renderer.toneMapping = THREE.ACESFilmicToneMapping
  renderer.toneMappingExposure = 1.2

  // ── Scene ──
  const scene = new THREE.Scene()

  // ── Camera ──
  const camera = new THREE.PerspectiveCamera(50, width / height, 0.1, 100)
  camera.position.set(8, 5, 10)
  camera.lookAt(0, 3.5, 0)

  // ── Lights ──
  const ambient = new THREE.AmbientLight(0x3a2a3a, 0.6)
  scene.add(ambient)

  const pointLight = new THREE.PointLight(COLORS.roseGlow, 1.5, 30)
  pointLight.position.set(3, 9, 5)
  scene.add(pointLight)

  const pointLight2 = new THREE.PointLight(COLORS.frost, 0.8, 25)
  pointLight2.position.set(-4, 2, 6)
  scene.add(pointLight2)

  // ── Tier rings ──
  const tierMeshes = []
  for (const tier of TIER_DEFS) {
    const ringGeo = new THREE.TorusGeometry(tier.radius, 0.04, 8, 64)
    const ringMat = new THREE.MeshStandardMaterial({
      color: tier.color,
      emissive: tier.color,
      emissiveIntensity: tier.opacity * 0.5,
      transparent: true,
      opacity: tier.opacity * 0.7,
    })
    const ring = new THREE.Mesh(ringGeo, ringMat)
    ring.rotation.x = Math.PI / 2
    ring.position.y = tier.y
    ring.userData = { tierDef: tier }
    scene.add(ring)

    // Label -- small plane with canvas texture
    const labelCanvas = document.createElement('canvas')
    labelCanvas.width = 256
    labelCanvas.height = 48
    const lctx = labelCanvas.getContext('2d')
    lctx.clearRect(0, 0, 256, 48)
    lctx.font = '600 22px ui-sans-serif, system-ui, sans-serif'
    lctx.fillStyle = `#${tier.color.toString(16).padStart(6, '0')}`
    lctx.textAlign = 'center'
    lctx.fillText(tier.name.toUpperCase(), 128, 30)

    const labelTex = new THREE.CanvasTexture(labelCanvas)
    const labelMat = new THREE.SpriteMaterial({
      map: labelTex,
      transparent: true,
      opacity: tier.opacity * 0.9,
      depthTest: false,
    })
    const label = new THREE.Sprite(labelMat)
    label.position.set(tier.radius + 1.2, tier.y + 0.1, 0)
    label.scale.set(2.4, 0.45, 1)
    scene.add(label)

    // Faint disc platform
    const discGeo = new THREE.CircleGeometry(tier.radius, 48)
    const discMat = new THREE.MeshStandardMaterial({
      color: tier.color,
      emissive: tier.color,
      emissiveIntensity: 0.1,
      transparent: true,
      opacity: tier.opacity * 0.08,
      side: THREE.DoubleSide,
    })
    const disc = new THREE.Mesh(discGeo, discMat)
    disc.rotation.x = -Math.PI / 2
    disc.position.y = tier.y
    scene.add(disc)

    // Golden outer glow for Persistent
    if (tier.name === 'Persistent') {
      const glowRingGeo = new THREE.TorusGeometry(tier.radius + 0.15, 0.06, 8, 64)
      const glowRingMat = new THREE.MeshStandardMaterial({
        color: COLORS.gold,
        emissive: COLORS.gold,
        emissiveIntensity: 0.8,
        transparent: true,
        opacity: 0.5,
      })
      const glowRing = new THREE.Mesh(glowRingGeo, glowRingMat)
      glowRing.rotation.x = Math.PI / 2
      glowRing.position.y = tier.y
      scene.add(glowRing)
    }

    tierMeshes.push(ring)
  }

  // ── Signal orbs array ──
  const signals = []
  const trailParticles = []

  // Shared geometries for instancing
  const orbGeo = new THREE.SphereGeometry(1, 16, 12)

  function createSignalOrb(tierIndex) {
    const tier = TIER_DEFS[tierIndex]
    const angle = Math.random() * Math.PI * 2
    const dist = (0.3 + Math.random() * 0.6) * tier.radius
    const orbMat = new THREE.MeshStandardMaterial({
      color: tier.color,
      emissive: tier.color,
      emissiveIntensity: 0.6,
      transparent: true,
      opacity: tier.opacity,
    })
    const orb = new THREE.Mesh(orbGeo, orbMat)
    orb.scale.setScalar(tier.signalSize)
    orb.position.set(
      Math.cos(angle) * dist,
      tier.y + (Math.random() - 0.5) * 0.3,
      Math.sin(angle) * dist,
    )
    scene.add(orb)

    const signal = {
      mesh: orb,
      tierIndex,
      angle,
      dist,
      orbitSpeed: 0.15 + Math.random() * 0.25,
      bobSpeed: 0.8 + Math.random() * 0.5,
      bobPhase: Math.random() * Math.PI * 2,
      score: 0.2 + Math.random() * 0.8,
      age: 0,
      alpha: tier.opacity,
      decayRate: 0.005 + Math.random() * 0.015,
      lastReinforcement: 0,
      state: 'alive',         // alive | graduating | decaying | dead
      graduateTarget: -1,
      graduateProgress: 0,
    }
    orb.userData = { signal }
    return signal
  }

  function spawnTrailParticle(x, y, z, color) {
    const geo = new THREE.SphereGeometry(0.03, 4, 4)
    const mat = new THREE.MeshBasicMaterial({
      color,
      transparent: true,
      opacity: 0.8,
    })
    const mesh = new THREE.Mesh(geo, mat)
    mesh.position.set(x, y, z)
    scene.add(mesh)
    trailParticles.push({
      mesh,
      life: 1.0,
      vy: 0.02 + Math.random() * 0.02,
      vx: (Math.random() - 0.5) * 0.01,
      vz: (Math.random() - 0.5) * 0.01,
    })
  }

  // Seed initial signals
  for (let i = 0; i < 5; i++) signals.push(createSignalOrb(0))
  for (let i = 0; i < 3; i++) signals.push(createSignalOrb(1))
  for (let i = 0; i < 2; i++) signals.push(createSignalOrb(2))
  signals.push(createSignalOrb(3))

  // ── Bloom post-processing (manual glow via additive blending planes) ──
  // Instead of a full EffectComposer, we add subtle glow sprites behind bright objects
  function addGlowSprite(target, color, scale) {
    const glowMat = new THREE.SpriteMaterial({
      color,
      transparent: true,
      opacity: 0.2,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
    })
    const glow = new THREE.Sprite(glowMat)
    glow.scale.setScalar(scale)
    target.add(glow)
    return glow
  }

  // Add glow to existing high-tier signals
  for (const sig of signals) {
    if (sig.tierIndex >= 2) {
      const tier = TIER_DEFS[sig.tierIndex]
      addGlowSprite(sig.mesh, tier.color, tier.signalSize * 5)
    }
  }

  // ── Raycaster for interaction ──
  const raycaster = new THREE.Raycaster()
  const mouse = new THREE.Vector2(-999, -999)
  let mouseClient = { x: 0, y: 0 }

  function onPointerMove(ev) {
    const rect = canvas.getBoundingClientRect()
    mouse.x = ((ev.clientX - rect.left) / rect.width) * 2 - 1
    mouse.y = -((ev.clientY - rect.top) / rect.height) * 2 + 1
    mouseClient = { x: ev.clientX - rect.left, y: ev.clientY - rect.top }
  }

  function onClick(ev) {
    const rect = canvas.getBoundingClientRect()
    const mx = ((ev.clientX - rect.left) / rect.width) * 2 - 1
    const my = -((ev.clientY - rect.top) / rect.height) * 2 + 1
    raycaster.setFromCamera(new THREE.Vector2(mx, my), camera)

    // Check tier rings
    const tierHits = raycaster.intersectObjects(tierMeshes)
    if (tierHits.length > 0) {
      const td = tierHits[0].object.userData.tierDef
      selectedTier.value = td
      tierInfoStyle.left = Math.min(ev.clientX - rootEl.value.getBoundingClientRect().left, root.clientWidth - 220) + 'px'
      tierInfoStyle.top = (ev.clientY - rootEl.value.getBoundingClientRect().top - 90) + 'px'
      return
    }
    selectedTier.value = null
  }

  canvas.addEventListener('pointermove', onPointerMove)
  canvas.addEventListener('click', onClick)

  // Simple orbit rotation via drag
  let isDragging = false
  let prevDragX = 0
  let orbitAngle = 0.6

  function onPointerDown(ev) {
    isDragging = true
    prevDragX = ev.clientX
  }
  function onPointerUp() { isDragging = false }
  function onDrag(ev) {
    if (!isDragging) return
    const dx = ev.clientX - prevDragX
    orbitAngle += dx * 0.005
    prevDragX = ev.clientX
  }

  canvas.addEventListener('pointerdown', onPointerDown)
  window.addEventListener('pointerup', onPointerUp)
  window.addEventListener('pointermove', onDrag)

  // ── Spawn timer ──
  let spawnTimer = 0

  // ── Store state for cleanup ──
  threeState.value = {
    THREE,
    renderer,
    scene,
    camera,
    signals,
    trailParticles,
    tierMeshes,
    raycaster,
    mouse,
    orbGeo,
    createSignalOrb,
    spawnTrailParticle,
    addGlowSprite,
    orbitAngle,
    spawnTimer,
    mouseClient,
  }

  // ── Animation loop ──
  let lastTime = performance.now()
  const clock = new THREE.Clock()

  function animate() {
    animationId = requestAnimationFrame(animate)
    if (!isVisible || prefersReducedMotion) return

    const st = threeState.value
    if (!st) return

    const rawDt = clock.getDelta()
    const dt = rawDt * timeScale.value
    const elapsed = clock.elapsedTime

    // Orbit camera
    if (!isDragging) {
      st.orbitAngle += dt * 0.08
    }
    const camDist = 13
    const camHeight = 5
    camera.position.x = Math.sin(st.orbitAngle) * camDist
    camera.position.z = Math.cos(st.orbitAngle) * camDist
    camera.position.y = camHeight
    camera.lookAt(0, 3.5, 0)

    // Pulse lights
    pointLight.intensity = 1.5 + Math.sin(elapsed * 0.7) * 0.3

    // Spawn new transient signals periodically
    st.spawnTimer += dt
    if (st.spawnTimer > 2.0 && st.signals.length < 30) {
      st.spawnTimer = 0
      const newSig = st.createSignalOrb(0)
      st.signals.push(newSig)
    }

    // Update signals
    let aliveCount = 0
    for (let i = st.signals.length - 1; i >= 0; i--) {
      const sig = st.signals[i]
      if (sig.state === 'dead') {
        scene.remove(sig.mesh)
        if (sig.mesh.material) sig.mesh.material.dispose()
        st.signals.splice(i, 1)
        continue
      }

      aliveCount++
      sig.age += dt

      const tier = TIER_DEFS[sig.tierIndex]

      if (sig.state === 'alive') {
        // Orbit
        sig.angle += sig.orbitSpeed * dt
        const baseY = tier.y
        sig.mesh.position.x = Math.cos(sig.angle) * sig.dist
        sig.mesh.position.z = Math.sin(sig.angle) * sig.dist
        sig.mesh.position.y = baseY + Math.sin(elapsed * sig.bobSpeed + sig.bobPhase) * 0.15

        // Ebbinghaus-style decay: alpha decreases over time
        sig.alpha -= sig.decayRate * dt
        sig.alpha = Math.max(sig.alpha, 0)
        sig.mesh.material.opacity = sig.alpha

        // Random reinforcement events
        if (sig.age - sig.lastReinforcement > 3.0 + Math.random() * 4.0) {
          if (Math.random() < 0.4) {
            // Reinforcement: brighten
            sig.lastReinforcement = sig.age
            sig.alpha = Math.min(sig.alpha + 0.3, tier.opacity)
            sig.score = Math.min(sig.score + 0.15, 1.0)
            sig.mesh.material.emissiveIntensity = 1.2
            setTimeout(() => {
              if (sig.mesh && sig.mesh.material) {
                sig.mesh.material.emissiveIntensity = 0.6
              }
            }, 500)
          }
        }

        // Graduation check
        if (sig.tierIndex < 3 && sig.score > 0.6 && sig.age > 4.0 && Math.random() < 0.003 * timeScale.value) {
          sig.state = 'graduating'
          sig.graduateTarget = sig.tierIndex + 1
          sig.graduateProgress = 0
        }

        // Decay death
        if (sig.alpha <= 0.02 && sig.tierIndex < 2) {
          sig.state = 'decaying'
        }
      } else if (sig.state === 'graduating') {
        // Float upward to next tier
        sig.graduateProgress += dt * 0.5
        const fromTier = TIER_DEFS[sig.tierIndex]
        const toTier = TIER_DEFS[sig.graduateTarget]
        const t = Math.min(sig.graduateProgress, 1.0)
        const eased = t * t * (3 - 2 * t)  // smoothstep

        sig.mesh.position.y = fromTier.y + (toTier.y - fromTier.y) * eased

        // Grow size
        const fromSize = fromTier.signalSize
        const toSize = toTier.signalSize
        sig.mesh.scale.setScalar(fromSize + (toSize - fromSize) * eased)

        // Color transition
        const fromColor = new THREE.Color(fromTier.color)
        const toColor = new THREE.Color(toTier.color)
        const lerpedColor = fromColor.lerp(toColor, eased)
        sig.mesh.material.color.copy(lerpedColor)
        sig.mesh.material.emissive.copy(lerpedColor)

        // Increase opacity
        sig.alpha = fromTier.opacity + (toTier.opacity - fromTier.opacity) * eased
        sig.mesh.material.opacity = sig.alpha

        // Spawn trail particles
        if (Math.random() < 0.3) {
          st.spawnTrailParticle(
            sig.mesh.position.x + (Math.random() - 0.5) * 0.2,
            sig.mesh.position.y - 0.1,
            sig.mesh.position.z + (Math.random() - 0.5) * 0.2,
            toTier.color,
          )
        }

        // Complete graduation
        if (t >= 1.0) {
          sig.tierIndex = sig.graduateTarget
          sig.state = 'alive'
          sig.age = 0
          sig.alpha = toTier.opacity
          sig.decayRate = sig.decayRate * 0.6  // slower decay at higher tiers
          sig.dist = (0.3 + Math.random() * 0.6) * toTier.radius
          sig.mesh.material.emissiveIntensity = 1.0
          setTimeout(() => {
            if (sig.mesh && sig.mesh.material) {
              sig.mesh.material.emissiveIntensity = 0.6
            }
          }, 800)

          // Add glow for higher tiers
          if (sig.tierIndex >= 2) {
            st.addGlowSprite(sig.mesh, toTier.color, toTier.signalSize * 5)
          }
        }
      } else if (sig.state === 'decaying') {
        // Sink below tier and fade
        sig.mesh.position.y -= dt * 0.8
        sig.alpha -= dt * 0.5
        sig.mesh.material.opacity = Math.max(sig.alpha, 0)
        sig.mesh.scale.multiplyScalar(0.99)
        sig.mesh.material.color.set(COLORS.decay)
        sig.mesh.material.emissive.set(COLORS.decay)

        if (sig.alpha <= 0) {
          sig.state = 'dead'
        }
      }
    }

    signalCount.value = aliveCount

    // Update trail particles
    for (let i = st.trailParticles.length - 1; i >= 0; i--) {
      const tp = st.trailParticles[i]
      tp.life -= dt * 1.5
      tp.mesh.position.y += tp.vy
      tp.mesh.position.x += tp.vx
      tp.mesh.position.z += tp.vz
      tp.mesh.material.opacity = Math.max(tp.life, 0)
      tp.mesh.scale.setScalar(tp.life * 0.8 + 0.2)

      if (tp.life <= 0) {
        scene.remove(tp.mesh)
        tp.mesh.material.dispose()
        tp.mesh.geometry.dispose()
        st.trailParticles.splice(i, 1)
      }
    }

    // Rotate tier rings slowly
    for (const ring of st.tierMeshes) {
      ring.rotation.z += dt * 0.1
    }

    // Hover detection
    raycaster.setFromCamera(mouse, camera)
    const orbMeshes = st.signals
      .filter(s => s.state !== 'dead')
      .map(s => s.mesh)
    const hits = raycaster.intersectObjects(orbMeshes)
    if (hits.length > 0 && hits[0].object.userData.signal) {
      const sig = hits[0].object.userData.signal
      hoveredSignal.value = {
        tierName: TIER_DEFS[sig.tierIndex].name,
        score: sig.score,
        ageSec: sig.age,
        state: sig.state === 'graduating'
          ? 'Graduating...'
          : sig.state === 'decaying'
            ? 'Decaying...'
            : sig.alpha < 0.3
              ? 'Fading'
              : 'Active',
      }
      tooltipStyle.left = mouseClient.x + 12 + 'px'
      tooltipStyle.top = mouseClient.y - 40 + 'px'
      canvas.style.cursor = 'pointer'
    } else {
      hoveredSignal.value = null
      canvas.style.cursor = isDragging ? 'grabbing' : 'grab'
    }

    renderer.render(scene, camera)
  }

  animate()

  // ── Resize handler ──
  let resizeTimer = null
  function onResize() {
    if (resizeTimer) clearTimeout(resizeTimer)
    resizeTimer = setTimeout(() => {
      const w = root.clientWidth
      const h = 400
      renderer.setSize(w, h)
      camera.aspect = w / h
      camera.updateProjectionMatrix()
    }, 150)
  }
  window.addEventListener('resize', onResize)

  // Visibility
  function onVisibility() { isVisible = !document.hidden }
  document.addEventListener('visibilitychange', onVisibility)

  // Store cleanup refs
  threeState.value._cleanup = {
    onPointerMove,
    onClick,
    onPointerDown,
    onPointerUp,
    onDrag,
    onResize,
    onVisibility,
    resizeTimer,
  }
})

onUnmounted(() => {
  if (animationId) {
    cancelAnimationFrame(animationId)
    animationId = null
  }

  const st = threeState.value
  if (!st) return

  const cleanup = st._cleanup
  if (cleanup) {
    const canvas = canvasEl.value
    if (canvas) {
      canvas.removeEventListener('pointermove', cleanup.onPointerMove)
      canvas.removeEventListener('click', cleanup.onClick)
      canvas.removeEventListener('pointerdown', cleanup.onPointerDown)
    }
    window.removeEventListener('pointerup', cleanup.onPointerUp)
    window.removeEventListener('pointermove', cleanup.onDrag)
    window.removeEventListener('resize', cleanup.onResize)
    document.removeEventListener('visibilitychange', cleanup.onVisibility)
    if (cleanup.resizeTimer) clearTimeout(cleanup.resizeTimer)
  }

  // Dispose Three.js objects
  st.renderer.dispose()
  st.scene.traverse((obj) => {
    if (obj.geometry) obj.geometry.dispose()
    if (obj.material) {
      if (obj.material.map) obj.material.map.dispose()
      obj.material.dispose()
    }
  })
  st.orbGeo.dispose()

  threeState.value = null
})
</script>

<style scoped>
.signal-lifecycle-3d {
  position: relative;
  width: 100%;
  height: 400px;
  border-radius: 8px;
  overflow: hidden;
  margin: 1.5rem 0;
  border: 1px solid var(--vp-c-divider, #2a2535);
  background: transparent;
}

.signal-lifecycle-3d canvas {
  width: 100%;
  height: 100%;
  display: block;
  cursor: grab;
}

.signal-lifecycle-3d canvas:active {
  cursor: grabbing;
}

/* ── Tier info overlay ── */
.tier-info {
  position: absolute;
  z-index: 10;
  padding: 10px 14px;
  background: rgba(10, 10, 15, 0.92);
  border: 1px solid var(--vp-c-divider, #2a2535);
  border-radius: 8px;
  pointer-events: none;
  max-width: 220px;
  backdrop-filter: blur(8px);
}

.tier-info__name {
  font-weight: 700;
  font-size: 14px;
  color: var(--vp-c-brand-1, #c77d8f);
  margin-bottom: 4px;
}

.tier-info__desc {
  font-size: 12px;
  line-height: 1.5;
  color: var(--vp-c-text-2, #9ca3af);
  margin-bottom: 4px;
}

.tier-info__retention {
  font-size: 11px;
  color: var(--vp-c-text-3, #6b7280);
  font-style: italic;
}

/* ── Signal hover tooltip ── */
.signal-tooltip {
  position: absolute;
  z-index: 10;
  padding: 6px 10px;
  background: rgba(10, 10, 15, 0.92);
  border: 1px solid var(--vp-c-divider, #2a2535);
  border-radius: 6px;
  pointer-events: none;
  white-space: nowrap;
  backdrop-filter: blur(8px);
}

.signal-tooltip__tier {
  font-weight: 600;
  font-size: 12px;
  color: var(--vp-c-brand-1, #c77d8f);
}

.signal-tooltip__score,
.signal-tooltip__age {
  font-size: 11px;
  color: var(--vp-c-text-2, #9ca3af);
}

.signal-tooltip__state {
  font-size: 10px;
  color: var(--vp-c-text-3, #6b7280);
  font-style: italic;
}

/* ── Time controls ── */
.lifecycle-controls {
  position: absolute;
  bottom: 10px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  background: rgba(10, 10, 15, 0.85);
  border: 1px solid var(--vp-c-divider, #2a2535);
  border-radius: 20px;
  backdrop-filter: blur(8px);
}

.ctrl-btn {
  padding: 3px 10px;
  border: 1px solid var(--vp-c-divider, #2a2535);
  border-radius: 12px;
  background: transparent;
  color: var(--vp-c-text-2, #9ca3af);
  cursor: pointer;
  font-size: 11px;
  font-weight: 500;
  transition: all 0.15s;
}

.ctrl-btn:hover {
  background: rgba(199, 125, 143, 0.15);
  color: var(--vp-c-brand-1, #c77d8f);
  border-color: var(--vp-c-brand-1, #c77d8f);
}

.ctrl-btn.active {
  background: rgba(199, 125, 143, 0.2);
  color: var(--vp-c-brand-1, #c77d8f);
  border-color: var(--vp-c-brand-1, #c77d8f);
}

.ctrl-label {
  font-size: 11px;
  color: var(--vp-c-text-3, #6b7280);
  margin-left: 6px;
  font-variant-numeric: tabular-nums;
}
</style>
