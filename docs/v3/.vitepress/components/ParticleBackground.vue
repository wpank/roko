<template>
  <canvas
    ref="canvasRef"
    class="particle-background"
    aria-hidden="true"
  />
</template>

<script setup>
import { ref, onMounted, onUnmounted } from 'vue'

const canvasRef = ref(null)

// ROSEDUST palette
const ROSE_DIM = { r: 139, g: 94, b: 107 }  // #8b5e6b
const ROSE     = { r: 199, g: 125, b: 143 }  // #c77d8f

// Tuning constants
const DESKTOP_COUNT = 80
const MOBILE_COUNT = 30
const MOBILE_BREAKPOINT = 768
const CONNECT_DIST = 120
const CURSOR_DIST = 200
const MAX_CONNECTIONS = 3
const FRAME_INTERVAL = 1000 / 30  // 30 fps
const MIN_SPEED = 0.1
const MAX_SPEED = 0.3
const PARTICLE_RADIUS_MIN = 1
const PARTICLE_RADIUS_MAX = 2
const CURSOR_FORCE = 0.15

let animationId = null
let lastFrameTime = 0
let particles = []
let mouse = { x: -9999, y: -9999 }
let isVisible = true
let prefersReducedMotion = false
// Track CSS-pixel dimensions (pre-DPR) for physics/drawing
let cssWidth = 0
let cssHeight = 0

function createParticle(w, h) {
  const angle = Math.random() * Math.PI * 2
  const speed = MIN_SPEED + Math.random() * (MAX_SPEED - MIN_SPEED)
  return {
    x: Math.random() * w,
    y: Math.random() * h,
    vx: Math.cos(angle) * speed,
    vy: Math.sin(angle) * speed,
    r: PARTICLE_RADIUS_MIN + Math.random() * (PARTICLE_RADIUS_MAX - PARTICLE_RADIUS_MIN),
  }
}

function initParticles(w, h) {
  const count = w < MOBILE_BREAKPOINT ? MOBILE_COUNT : DESKTOP_COUNT
  particles = []
  for (let i = 0; i < count; i++) {
    particles.push(createParticle(w, h))
  }
}

function updateParticles(w, h) {
  for (let i = 0; i < particles.length; i++) {
    const p = particles[i]

    // Mouse interaction: gentle drift toward/away from cursor
    const dx = mouse.x - p.x
    const dy = mouse.y - p.y
    const distToCursor = Math.sqrt(dx * dx + dy * dy)
    if (distToCursor < CURSOR_DIST && distToCursor > 0) {
      const force = (1 - distToCursor / CURSOR_DIST) * CURSOR_FORCE
      // Particles within inner 40% are pushed away, outer 60% are pulled in
      const innerThreshold = CURSOR_DIST * 0.4
      if (distToCursor < innerThreshold) {
        p.vx -= (dx / distToCursor) * force
        p.vy -= (dy / distToCursor) * force
      } else {
        p.vx += (dx / distToCursor) * force * 0.3
        p.vy += (dy / distToCursor) * force * 0.3
      }
    }

    // Clamp velocity to max speed
    const currentSpeed = Math.sqrt(p.vx * p.vx + p.vy * p.vy)
    if (currentSpeed > MAX_SPEED) {
      p.vx = (p.vx / currentSpeed) * MAX_SPEED
      p.vy = (p.vy / currentSpeed) * MAX_SPEED
    }
    // Enforce minimum speed so particles never stall
    if (currentSpeed < MIN_SPEED && currentSpeed > 0) {
      p.vx = (p.vx / currentSpeed) * MIN_SPEED
      p.vy = (p.vy / currentSpeed) * MIN_SPEED
    }

    p.x += p.vx
    p.y += p.vy

    // Wrap around edges
    if (p.x < -10) p.x = w + 10
    else if (p.x > w + 10) p.x = -10
    if (p.y < -10) p.y = h + 10
    else if (p.y > h + 10) p.y = -10
  }
}

function draw(ctx, w, h) {
  ctx.clearRect(0, 0, w, h)

  const connectDistSq = CONNECT_DIST * CONNECT_DIST
  const cursorDistSq = CURSOR_DIST * CURSOR_DIST

  // Track connection count per particle
  const connectionCounts = new Uint8Array(particles.length)

  // Draw connections
  for (let i = 0; i < particles.length; i++) {
    if (connectionCounts[i] >= MAX_CONNECTIONS) continue
    const a = particles[i]

    for (let j = i + 1; j < particles.length; j++) {
      if (connectionCounts[j] >= MAX_CONNECTIONS) continue

      const dx = a.x - particles[j].x
      const dy = a.y - particles[j].y
      const distSq = dx * dx + dy * dy

      if (distSq < connectDistSq) {
        const dist = Math.sqrt(distSq)
        const t = 1 - dist / CONNECT_DIST  // 1 at center, 0 at edge

        // Check if this connection is near cursor for brighter highlight
        const midX = (a.x + particles[j].x) / 2
        const midY = (a.y + particles[j].y) / 2
        const cdx = mouse.x - midX
        const cdy = mouse.y - midY
        const cursorMidDistSq = cdx * cdx + cdy * cdy
        const nearCursor = cursorMidDistSq < cursorDistSq

        if (nearCursor) {
          const cursorT = 1 - Math.sqrt(cursorMidDistSq) / CURSOR_DIST
          const alpha = t * (0.15 + cursorT * 0.05)  // up to ~20%
          ctx.strokeStyle = `rgba(${ROSE.r}, ${ROSE.g}, ${ROSE.b}, ${alpha})`
        } else {
          const alpha = t * 0.15  // max 15%
          ctx.strokeStyle = `rgba(${ROSE_DIM.r}, ${ROSE_DIM.g}, ${ROSE_DIM.b}, ${alpha})`
        }

        ctx.lineWidth = 0.5
        ctx.beginPath()
        ctx.moveTo(a.x, a.y)
        ctx.lineTo(particles[j].x, particles[j].y)
        ctx.stroke()

        connectionCounts[i]++
        connectionCounts[j]++
      }
    }
  }

  // Draw particles
  for (let i = 0; i < particles.length; i++) {
    const p = particles[i]

    // Check proximity to cursor for color
    const dx = mouse.x - p.x
    const dy = mouse.y - p.y
    const distSq = dx * dx + dy * dy
    const nearCursor = distSq < cursorDistSq

    if (nearCursor) {
      const t = 1 - Math.sqrt(distSq) / CURSOR_DIST
      const alpha = 0.3 + t * 0.15
      ctx.fillStyle = `rgba(${ROSE.r}, ${ROSE.g}, ${ROSE.b}, ${alpha})`
    } else {
      ctx.fillStyle = `rgba(${ROSE_DIM.r}, ${ROSE_DIM.g}, ${ROSE_DIM.b}, 0.3)`
    }

    ctx.beginPath()
    ctx.arc(p.x, p.y, p.r, 0, Math.PI * 2)
    ctx.fill()
  }
}

function loop(timestamp) {
  animationId = requestAnimationFrame(loop)

  if (!isVisible || prefersReducedMotion) return

  // Throttle to 30 fps
  if (timestamp - lastFrameTime < FRAME_INTERVAL) return
  lastFrameTime = timestamp

  const canvas = canvasRef.value
  if (!canvas) return

  const ctx = canvas.getContext('2d')
  if (!ctx) return

  updateParticles(cssWidth, cssHeight)
  draw(ctx, cssWidth, cssHeight)
}

function handleResize() {
  const canvas = canvasRef.value
  if (!canvas) return

  const dpr = window.devicePixelRatio || 1
  const w = window.innerWidth
  const h = window.innerHeight

  canvas.width = w * dpr
  canvas.height = h * dpr
  canvas.style.width = w + 'px'
  canvas.style.height = h + 'px'

  const ctx = canvas.getContext('2d')
  if (ctx) {
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
  }

  // Store CSS-pixel dimensions for physics/drawing
  cssWidth = w
  cssHeight = h

  // Reinitialize particles for new dimensions
  initParticles(w, h)
}

function handleMouseMove(e) {
  mouse.x = e.clientX
  mouse.y = e.clientY
}

function handleMouseLeave() {
  mouse.x = -9999
  mouse.y = -9999
}

function handleVisibilityChange() {
  isVisible = !document.hidden
}

let resizeTimer = null
function handleResizeDebounced() {
  if (resizeTimer) clearTimeout(resizeTimer)
  resizeTimer = setTimeout(handleResize, 150)
}

let motionQuery = null

onMounted(() => {
  // Check reduced motion preference
  motionQuery = window.matchMedia('(prefers-reduced-motion: reduce)')
  prefersReducedMotion = motionQuery.matches
  motionQuery.addEventListener('change', (e) => {
    prefersReducedMotion = e.matches
  })

  if (prefersReducedMotion) return

  handleResize()

  window.addEventListener('resize', handleResizeDebounced)
  window.addEventListener('mousemove', handleMouseMove)
  document.addEventListener('mouseleave', handleMouseLeave)
  document.addEventListener('visibilitychange', handleVisibilityChange)

  animationId = requestAnimationFrame(loop)
})

onUnmounted(() => {
  if (animationId) {
    cancelAnimationFrame(animationId)
    animationId = null
  }
  if (resizeTimer) {
    clearTimeout(resizeTimer)
    resizeTimer = null
  }

  window.removeEventListener('resize', handleResizeDebounced)
  window.removeEventListener('mousemove', handleMouseMove)
  document.removeEventListener('mouseleave', handleMouseLeave)
  document.removeEventListener('visibilitychange', handleVisibilityChange)
})
</script>

<style scoped>
.particle-background {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  z-index: -1;
  pointer-events: none;
}
</style>
