<template>
  <div class="crate-graph-3d" ref="rootEl">
    <!-- Loading state -->
    <div v-if="!ready" class="cg3d-loading">
      <div class="cg3d-spinner"></div>
      <span>Loading 3D graph...</span>
    </div>

    <!-- Three.js canvas mount point -->
    <div ref="canvasContainer" class="cg3d-canvas"></div>

    <!-- CSS2D label overlay (Three.js CSS2DRenderer mounts here) -->
    <div ref="labelContainer" class="cg3d-labels"></div>

    <!-- Legend overlay -->
    <div class="cg3d-legend">
      <span
        v-for="tier in tierDefs"
        :key="tier.id"
        class="cg3d-legend-item"
      >
        <span class="cg3d-legend-dot" :style="{ background: tier.color }"></span>
        {{ tier.label }}
      </span>
    </div>

    <!-- Controls hint -->
    <div class="cg3d-hint" v-if="ready && !interacted">
      Drag to rotate &middot; Scroll to zoom &middot; Click a crate for details
    </div>

    <!-- Info panel -->
    <transition name="cg3d-panel">
      <div v-if="selectedCrate" class="cg3d-panel" @click.stop>
        <button class="cg3d-panel-close" @click="selectedCrate = null">&times;</button>
        <h3 class="cg3d-panel-name">{{ selectedCrate.name }}</h3>
        <p class="cg3d-panel-desc">{{ selectedCrate.description }}</p>

        <div class="cg3d-panel-stats">
          <div class="cg3d-stat">
            <span class="cg3d-stat-val">{{ formatLoc(selectedCrate.loc) }}</span>
            <span class="cg3d-stat-lbl">Lines of Code</span>
          </div>
          <div class="cg3d-stat">
            <span class="cg3d-stat-val">{{ selectedCrate.tests.toLocaleString() }}</span>
            <span class="cg3d-stat-lbl">Tests</span>
          </div>
          <div class="cg3d-stat">
            <span class="cg3d-stat-val">{{ tierLabel(selectedCrate.tier) }}</span>
            <span class="cg3d-stat-lbl">Tier</span>
          </div>
        </div>

        <div v-if="selectedCrate.chapter" class="cg3d-panel-link">
          <a :href="selectedCrate.chapter">View documentation &rarr;</a>
        </div>

        <div v-if="selectedCrate.deps.length" class="cg3d-panel-deps">
          <h4>Dependencies ({{ selectedCrate.deps.length }})</h4>
          <div class="cg3d-dep-tags">
            <span
              v-for="dep in selectedCrate.deps"
              :key="dep"
              class="cg3d-dep-tag"
              :style="{ borderColor: getCrateColor(dep) }"
              @click="selectCrateById(dep)"
            >{{ dep.replace('roko-', '') }}</span>
          </div>
        </div>

        <div v-if="getDependents(selectedCrate.name).length" class="cg3d-panel-deps">
          <h4>Dependents ({{ getDependents(selectedCrate.name).length }})</h4>
          <div class="cg3d-dep-tags">
            <span
              v-for="dep in getDependents(selectedCrate.name)"
              :key="dep"
              class="cg3d-dep-tag cg3d-dep-tag--dependent"
              :style="{ borderColor: getCrateColor(dep) }"
              @click="selectCrateById(dep)"
            >{{ dep.replace('roko-', '') }}</span>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted, shallowRef, watch } from 'vue'

// ─── Tier definitions (ROSEDUST palette) ───
const tierDefs = [
  { id: 'kernel',    label: 'Kernel',    color: '#60a5fa' },
  { id: 'engine',    label: 'Engine',    color: '#5eead4' },
  { id: 'agent',     label: 'Agent',     color: '#fbbf24' },
  { id: 'cognitive', label: 'Cognitive', color: '#a78bfa' },
  { id: 'surface',   label: 'Surface',   color: '#c77d8f' },
  { id: 'infra',     label: 'Infra',     color: '#94a3b8' },
]

const tierColorMap = Object.fromEntries(tierDefs.map(t => [t.id, t.color]))

// ─── Crate data (static, workspace-derived) ───
const crateDataMap = {
  'roko-primitives':   { name: 'roko-primitives',   tier: 'kernel',    loc: 5241,   tests: 142,  description: 'HDC vectors, tier routing, and shared mathematical primitives.',              chapter: './01-SIGNAL.html', deps: [] },
  'roko-core':         { name: 'roko-core',         tier: 'kernel',    loc: 92820,  tests: 1849, description: 'Signal + 12 kernel traits. Config, tools, errors.',                          chapter: './01-SIGNAL.html', deps: ['roko-primitives'] },
  'roko-fs':           { name: 'roko-fs',           tier: 'kernel',    loc: 11848,  tests: 158,  description: 'Filesystem-backed Substrate: JSONL signal log, GC, layout.',                 chapter: './01-SIGNAL.html', deps: ['roko-core'] },
  'roko-std':          { name: 'roko-std',          tier: 'kernel',    loc: 9337,   tests: 218,  description: '35 standard tool definitions (16 local + 19 GitHub MCP).',                   chapter: './19-TOOLS-PLUGINS.html', deps: ['roko-core'] },
  'roko-eval':         { name: 'roko-eval',         tier: 'kernel',    loc: 2400,   tests: 45,   description: 'Evaluation framework: EvidenceCollector, Criterion, Profile.',                chapter: './07-GATES.html', deps: ['roko-core'] },
  'roko-graph':        { name: 'roko-graph',        tier: 'engine',    loc: 25686,  tests: 400,  description: 'Sole execution engine: DAG cells, topology, cost state, immune Graph.',       chapter: './03-GRAPH.html', deps: ['roko-core'] },
  'roko-gate':         { name: 'roko-gate',         tier: 'engine',    loc: 28704,  tests: 546,  description: '19 gates, 7-rung pipeline, adaptive thresholds.',                            chapter: './07-GATES.html', deps: ['roko-core', 'roko-agent', 'roko-std'] },
  'roko-plugin':       { name: 'roko-plugin',       tier: 'engine',    loc: 5665,   tests: 92,   description: 'Plugin SDK: signed manifests, WASM hooks, dependency resolution.',            chapter: './19-TOOLS-PLUGINS.html', deps: ['roko-core'] },
  'roko-compose':      { name: 'roko-compose',      tier: 'engine',    loc: 32500,  tests: 419,  description: 'Prompt assembly: 11 role templates, 9-layer SystemPromptBuilder.',            chapter: './06-COMPOSITION.html', deps: ['roko-agent', 'roko-core', 'roko-daimon', 'roko-dreams', 'roko-graph', 'roko-learn', 'roko-neuro', 'roko-std'] },
  'roko-conductor':    { name: 'roko-conductor',    tier: 'engine',    loc: 10665,  tests: 320,  description: '12 watchers, circuit breaker, diagnosis.',                                   chapter: './30-CONDUCTOR.html', deps: ['roko-core', 'roko-learn'] },
  'roko-gateway':      { name: 'roko-gateway',      tier: 'engine',    loc: 4858,   tests: 26,   description: 'Nine-stage inference gateway: routing, caching, cost accounting.',            chapter: './20-GATEWAY.html', deps: ['roko-core', 'roko-agent', 'roko-learn', 'roko-graph'] },
  'roko-runtime':      { name: 'roko-runtime',      tier: 'engine',    loc: 28535,  tests: 285,  description: 'ProcessSupervisor, event bus, cancellation tokens, workflow contracts.',       chapter: './04-EXECUTION.html', deps: ['roko-primitives', 'roko-core', 'roko-compose', 'roko-learn', 'roko-gate'] },
  'roko-execution':    { name: 'roko-execution',    tier: 'engine',    loc: 12036,  tests: 239,  description: 'RuntimeServices builder: shared service facade for CLI/serve/ACP.',           chapter: './04-EXECUTION.html', deps: ['roko-core', 'roko-agent', 'roko-gate', 'roko-learn', 'roko-compose', 'roko-graph', 'roko-runtime', 'roko-neuro', 'roko-fs'] },
  'roko-agent':        { name: 'roko-agent',        tier: 'agent',     loc: 107778, tests: 1721, description: '12 LLM provider kinds. Pools, MCP, tool loop, safety.',                     chapter: './05-AGENT.html', deps: ['roko-core', 'roko-fs', 'roko-graph', 'roko-std'] },
  'roko-daimon':       { name: 'roko-daimon',       tier: 'cognitive', loc: 8446,   tests: 104,  description: 'Affect engine: somatic markers, emotional state, dispatch modulation.',       chapter: './11-AFFECT.html', deps: ['roko-core'] },
  'roko-learn':        { name: 'roko-learn',        tier: 'cognitive', loc: 72888,  tests: 954,  description: 'Episodes, playbooks, bandits, model routing, A/B experiments.',              chapter: './08-LEARNING.html', deps: ['roko-core', 'roko-agent', 'roko-daimon', 'roko-fs', 'roko-primitives'] },
  'roko-neuro':        { name: 'roko-neuro',        tier: 'cognitive', loc: 22365,  tests: 243,  description: 'Durable knowledge store: tier progression, distillation, HDC fingerprinting.', chapter: './09-MEMORY.html', deps: ['roko-core', 'roko-fs', 'roko-agent', 'roko-learn'] },
  'roko-dreams':       { name: 'roko-dreams',       tier: 'cognitive', loc: 14545,  tests: 89,   description: 'Offline consolidation: hypnagogia, imagination cycles, dream journals.',     chapter: './10-DREAMS.html', deps: ['roko-core', 'roko-neuro', 'roko-learn', 'roko-agent', 'roko-primitives'] },
  'roko-chain':        { name: 'roko-chain',        tier: 'infra',     loc: 29683,  tests: 340,  description: 'Optional chain client: identity, delegation, marketplace, arena, DeFi.',      chapter: './23-PAYMENTS-ECONOMY.html', deps: ['roko-core'] },
  'roko-index':        { name: 'roko-index',        tier: 'infra',     loc: 5822,   tests: 88,   description: 'Code intelligence: parser, symbol graph, PageRank, HDC fingerprinting.',     chapter: './34-CODE-INTELLIGENCE.html', deps: ['roko-core', 'roko-primitives'] },
  'roko-agent-server': { name: 'roko-agent-server', tier: 'surface',   loc: 6308,   tests: 21,   description: 'Per-agent HTTP sidecar: /message, /stream WS, /predictions, /research.',     chapter: './27-ACP.html', deps: ['roko-agent', 'roko-core', 'roko-fs', 'roko-learn', 'roko-neuro'] },
  'roko-serve':        { name: 'roko-serve',        tier: 'surface',   loc: 101157, tests: 467,  description: 'HTTP control plane: REST routes + SSE + WebSocket.',           chapter: './26-HTTP-API.html', deps: ['roko-core', 'roko-agent', 'roko-agent-server', 'roko-learn', 'roko-neuro', 'roko-dreams', 'roko-gate', 'roko-gateway', 'roko-fs', 'roko-compose', 'roko-std', 'roko-plugin', 'roko-daimon', 'roko-runtime', 'roko-execution'] },
  'roko-acp':          { name: 'roko-acp',          tier: 'surface',   loc: 20817,  tests: 120,  description: 'Agent Client Protocol for Cursor/editor integration. 180 ACP tests.',        chapter: './27-ACP.html', deps: ['roko-core', 'roko-execution', 'roko-runtime', 'roko-agent', 'roko-gate', 'roko-compose', 'roko-serve', 'roko-learn', 'roko-dreams', 'roko-neuro'] },
  'roko-cli':          { name: 'roko-cli',          tier: 'surface',   loc: 264660, tests: 3109, description: 'Main binary: plan runner, merge queue, worktree manager, ratatui TUI.',      chapter: './28-CLI.html', deps: ['roko-core', 'roko-std', 'roko-fs', 'roko-learn', 'roko-compose', 'roko-agent', 'roko-agent-server', 'roko-gate', 'roko-dreams', 'roko-daimon', 'roko-neuro', 'roko-conductor', 'roko-plugin', 'roko-runtime', 'roko-serve'] },
  'roko-mcp-code':     { name: 'roko-mcp-code',     tier: 'infra',     loc: 3120,   tests: 34,   description: 'Code-intelligence MCP server for codebase understanding.',                   chapter: './34-CODE-INTELLIGENCE.html', deps: ['roko-core', 'roko-index'] },
  'roko-mcp-github':   { name: 'roko-mcp-github',   tier: 'infra',     loc: 2840,   tests: 22,   description: 'GitHub MCP integration: issues, PRs, actions workflows.',                    chapter: './19-TOOLS-PLUGINS.html', deps: ['roko-core'] },
  'roko-mcp-slack':    { name: 'roko-mcp-slack',    tier: 'infra',     loc: 1650,   tests: 14,   description: 'Slack MCP integration for team notifications.',                              chapter: './19-TOOLS-PLUGINS.html', deps: ['roko-core'] },
  'roko-mcp-scripts':  { name: 'roko-mcp-scripts',  tier: 'infra',     loc: 1280,   tests: 10,   description: 'Script-runner MCP server for local tool execution.',                         chapter: './19-TOOLS-PLUGINS.html', deps: ['roko-core'] },
  'roko-mcp-stdio':    { name: 'roko-mcp-stdio',    tier: 'infra',     loc: 960,    tests: 8,    description: 'Stdio MCP transport: subprocess tool communication.',                        chapter: './19-TOOLS-PLUGINS.html', deps: ['roko-core'] },
  'roko-lang-rust':    { name: 'roko-lang-rust',    tier: 'infra',     loc: 4200,   tests: 55,   description: 'Rust language support: parsing, symbol extraction, analysis.',                chapter: './34-CODE-INTELLIGENCE.html', deps: ['roko-core', 'roko-index'] },
  'roko-lang-typescript': { name: 'roko-lang-typescript', tier: 'infra', loc: 3100,  tests: 38,   description: 'TypeScript language support: parsing, symbol extraction.',                  chapter: './34-CODE-INTELLIGENCE.html', deps: ['roko-core', 'roko-index'] },
  'roko-lang-go':      { name: 'roko-lang-go',      tier: 'infra',     loc: 2600,   tests: 30,   description: 'Go language support: parsing, symbol extraction.',                           chapter: './34-CODE-INTELLIGENCE.html', deps: ['roko-core', 'roko-index'] },
  'roko-demo':         { name: 'roko-demo',          tier: 'surface',   loc: 3800,   tests: 12,   description: 'Demo binary for showcasing features and example workflows.',                 chapter: './28-CLI.html', deps: ['roko-core', 'roko-agent', 'roko-learn'] },
}

const crateList = Object.values(crateDataMap)

// ─── Refs ───
const rootEl = ref(null)
const canvasContainer = ref(null)
const labelContainer = ref(null)
const ready = ref(false)
const interacted = ref(false)
const selectedCrate = ref(null)

// Three.js refs (shallowRef to avoid Vue reactivity on heavy objects)
const threeState = shallowRef(null)

// ─── Helpers ───
function formatLoc(n) {
  if (n >= 1000) return (n / 1000).toFixed(1) + 'k'
  return String(n)
}

function tierLabel(tierId) {
  const tier = tierDefs.find(t => t.id === tierId)
  return tier ? tier.label : tierId
}

function getCrateColor(id) {
  const data = crateDataMap[id]
  return data ? tierColorMap[data.tier] || '#94a3b8' : '#94a3b8'
}

function getDependents(name) {
  return crateList
    .filter(c => c.deps.includes(name))
    .map(c => c.name)
}

function selectCrateById(id) {
  const data = crateDataMap[id]
  if (data) selectedCrate.value = data
}

// ─── Force-directed layout (simple Fruchterman-Reingold variant) ───
function computeForceLayout(crates) {
  const nodes = {}
  const tierYBase = {
    kernel: -60,
    engine: -20,
    agent: 20,
    cognitive: 50,
    surface: 80,
    infra: -90,
  }

  // Initialize positions: tier-based Y with jittered X/Z
  let i = 0
  const tierCounts = {}
  for (const c of crates) {
    const tier = c.tier
    tierCounts[tier] = (tierCounts[tier] || 0) + 1
    const idx = tierCounts[tier]
    const angle = (idx / (crates.filter(x => x.tier === tier).length + 1)) * Math.PI * 2
    const radius = 40 + Math.random() * 20

    nodes[c.name] = {
      x: Math.cos(angle) * radius + (Math.random() - 0.5) * 10,
      y: (tierYBase[tier] || 0) + (Math.random() - 0.5) * 15,
      z: Math.sin(angle) * radius + (Math.random() - 0.5) * 10,
      vx: 0, vy: 0, vz: 0,
    }
    i++
  }

  // Build edge list
  const edges = []
  for (const c of crates) {
    for (const dep of c.deps) {
      if (crateDataMap[dep]) {
        edges.push({ from: c.name, to: dep })
      }
    }
  }

  // Iterate force simulation
  const iterations = 300
  const repulsionStrength = 800
  const attractionStrength = 0.015
  const damping = 0.92
  const centerPull = 0.002

  for (let iter = 0; iter < iterations; iter++) {
    const temp = 1.0 - iter / iterations

    // Repulsion between all pairs
    const names = Object.keys(nodes)
    for (let a = 0; a < names.length; a++) {
      for (let b = a + 1; b < names.length; b++) {
        const na = nodes[names[a]]
        const nb = nodes[names[b]]
        let dx = na.x - nb.x
        let dy = na.y - nb.y
        let dz = na.z - nb.z
        let dist = Math.sqrt(dx * dx + dy * dy + dz * dz) + 0.1
        let force = repulsionStrength / (dist * dist)
        let fx = (dx / dist) * force * temp
        let fy = (dy / dist) * force * temp
        let fz = (dz / dist) * force * temp
        na.vx += fx; na.vy += fy; na.vz += fz
        nb.vx -= fx; nb.vy -= fy; nb.vz -= fz
      }
    }

    // Attraction along edges
    for (const e of edges) {
      const na = nodes[e.from]
      const nb = nodes[e.to]
      if (!na || !nb) continue
      let dx = nb.x - na.x
      let dy = nb.y - na.y
      let dz = nb.z - na.z
      let dist = Math.sqrt(dx * dx + dy * dy + dz * dz) + 0.1
      let force = dist * attractionStrength
      let fx = (dx / dist) * force
      let fy = (dy / dist) * force
      let fz = (dz / dist) * force
      na.vx += fx; na.vy += fy; na.vz += fz
      nb.vx -= fx; nb.vy -= fy; nb.vz -= fz
    }

    // Center gravity
    for (const name of names) {
      const n = nodes[name]
      n.vx -= n.x * centerPull
      n.vy -= n.y * centerPull
      n.vz -= n.z * centerPull
    }

    // Apply velocities + damping
    for (const name of names) {
      const n = nodes[name]
      n.vx *= damping; n.vy *= damping; n.vz *= damping
      n.x += n.vx; n.y += n.vy; n.z += n.vz
    }
  }

  return { nodes, edges }
}

// ─── Three.js scene setup (dynamic import for SSR safety) ───
async function initThreeScene() {
  if (!canvasContainer.value || !labelContainer.value) return

  const THREE = await import('three')
  const { OrbitControls } = await import('three/examples/jsm/controls/OrbitControls.js')
  const { EffectComposer } = await import('three/examples/jsm/postprocessing/EffectComposer.js')
  const { RenderPass } = await import('three/examples/jsm/postprocessing/RenderPass.js')
  const { UnrealBloomPass } = await import('three/examples/jsm/postprocessing/UnrealBloomPass.js')
  const { CSS2DRenderer, CSS2DObject } = await import('three/examples/jsm/renderers/CSS2DRenderer.js')

  const container = canvasContainer.value
  const width = container.clientWidth
  const height = container.clientHeight

  // ── Scene ──
  const scene = new THREE.Scene()
  scene.background = new THREE.Color(0x0a0a0f)

  // Subtle fog for depth
  scene.fog = new THREE.FogExp2(0x0a0a0f, 0.0025)

  // ── Camera ──
  const camera = new THREE.PerspectiveCamera(55, width / height, 0.1, 2000)
  camera.position.set(80, 60, 120)
  camera.lookAt(0, 0, 0)

  // ── WebGL Renderer ──
  const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: false })
  renderer.setSize(width, height)
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2))
  renderer.toneMapping = THREE.ACESFilmicToneMapping
  renderer.toneMappingExposure = 1.2
  container.appendChild(renderer.domElement)

  // ── CSS2D Renderer for labels ──
  const labelRenderer = new CSS2DRenderer()
  labelRenderer.setSize(width, height)
  labelRenderer.domElement.style.position = 'absolute'
  labelRenderer.domElement.style.top = '0'
  labelRenderer.domElement.style.left = '0'
  labelRenderer.domElement.style.pointerEvents = 'none'
  labelContainer.value.appendChild(labelRenderer.domElement)

  // ── Post-processing (bloom) ──
  const composer = new EffectComposer(renderer)
  const renderPass = new RenderPass(scene, camera)
  composer.addPass(renderPass)

  const bloomPass = new UnrealBloomPass(
    new THREE.Vector2(width, height),
    0.8,   // strength
    0.4,   // radius
    0.6    // threshold
  )
  composer.addPass(bloomPass)

  // ── OrbitControls ──
  const controls = new OrbitControls(camera, renderer.domElement)
  controls.enableDamping = true
  controls.dampingFactor = 0.05
  controls.minDistance = 30
  controls.maxDistance = 400
  controls.autoRotate = true
  controls.autoRotateSpeed = 0.3
  controls.enablePan = true
  controls.target.set(0, 0, 0)

  // Track interaction
  controls.addEventListener('start', () => {
    interacted.value = true
    controls.autoRotate = false
  })

  // Resume auto-rotate after idle
  let idleTimer = null
  controls.addEventListener('end', () => {
    clearTimeout(idleTimer)
    idleTimer = setTimeout(() => {
      controls.autoRotate = true
    }, 8000)
  })

  // ── Compute layout ──
  const layout = computeForceLayout(crateList)

  // ── Build node meshes ──
  const nodeGroup = new THREE.Group()
  scene.add(nodeGroup)

  // Map: crate name -> mesh, for raycasting
  const nodeMeshes = {}
  const nodeData = {}  // mesh.uuid -> crate data

  // Glow sprite texture (programmatic)
  const glowCanvas = document.createElement('canvas')
  glowCanvas.width = 64
  glowCanvas.height = 64
  const glowCtx = glowCanvas.getContext('2d')
  const gradient = glowCtx.createRadialGradient(32, 32, 0, 32, 32, 32)
  gradient.addColorStop(0, 'rgba(255,255,255,0.6)')
  gradient.addColorStop(0.4, 'rgba(255,255,255,0.15)')
  gradient.addColorStop(1, 'rgba(255,255,255,0)')
  glowCtx.fillStyle = gradient
  glowCtx.fillRect(0, 0, 64, 64)
  const glowTexture = new THREE.CanvasTexture(glowCanvas)

  for (const crate of crateList) {
    const pos = layout.nodes[crate.name]
    if (!pos) continue

    const color = new THREE.Color(tierColorMap[crate.tier] || '#94a3b8')

    // Node sphere: size ~ sqrt(LOC), clamped
    const baseSize = Math.sqrt(crate.loc) * 0.02
    const sphereSize = Math.max(1.2, Math.min(baseSize, 6))

    // Sphere mesh
    const geometry = new THREE.SphereGeometry(sphereSize, 24, 24)
    const material = new THREE.MeshStandardMaterial({
      color: color,
      emissive: color,
      emissiveIntensity: 0.5,
      metalness: 0.3,
      roughness: 0.4,
    })
    const mesh = new THREE.Mesh(geometry, material)
    mesh.position.set(pos.x, pos.y, pos.z)
    nodeGroup.add(mesh)

    nodeMeshes[crate.name] = mesh
    nodeData[mesh.uuid] = crate

    // Outer glow sprite
    const spriteMat = new THREE.SpriteMaterial({
      map: glowTexture,
      color: color,
      transparent: true,
      opacity: 0.35,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
    })
    const sprite = new THREE.Sprite(spriteMat)
    sprite.scale.set(sphereSize * 4, sphereSize * 4, 1)
    mesh.add(sprite)

    // CSS2D label
    const labelDiv = document.createElement('div')
    labelDiv.className = 'cg3d-node-label'
    labelDiv.textContent = crate.name.replace('roko-', '')
    labelDiv.style.color = tierColorMap[crate.tier] || '#94a3b8'
    const label = new CSS2DObject(labelDiv)
    label.position.set(0, sphereSize + 1.5, 0)
    mesh.add(label)
  }

  // ── Build edges ──
  const edgeGroup = new THREE.Group()
  scene.add(edgeGroup)

  const edgeMeshes = []  // { line, from, to }

  for (const edge of layout.edges) {
    const fromMesh = nodeMeshes[edge.from]
    const toMesh = nodeMeshes[edge.to]
    if (!fromMesh || !toMesh) continue

    const fromColor = new THREE.Color(getCrateColor(edge.from))
    const toColor = new THREE.Color(getCrateColor(edge.to))

    // Curved edge: quadratic bezier through midpoint pushed outward
    const from = fromMesh.position
    const to = toMesh.position
    const mid = new THREE.Vector3().addVectors(from, to).multiplyScalar(0.5)

    // Push midpoint outward slightly for curve
    const offset = new THREE.Vector3()
      .subVectors(from, to)
      .cross(new THREE.Vector3(0, 1, 0))
      .normalize()
      .multiplyScalar(3)
    mid.add(offset)
    mid.y += 2

    const curve = new THREE.QuadraticBezierCurve3(from, mid, to)
    const points = curve.getPoints(20)

    const geometry = new THREE.BufferGeometry().setFromPoints(points)

    // Per-vertex color: interpolate from source to target color
    const colors = []
    for (let i = 0; i <= 20; i++) {
      const t = i / 20
      const c = new THREE.Color().copy(fromColor).lerp(toColor, t)
      colors.push(c.r, c.g, c.b)
    }
    geometry.setAttribute('color', new THREE.Float32BufferAttribute(colors, 3))

    const material = new THREE.LineBasicMaterial({
      vertexColors: true,
      transparent: true,
      opacity: 0.25,
      linewidth: 1,
    })
    const line = new THREE.Line(geometry, material)
    edgeGroup.add(line)

    edgeMeshes.push({ line, from: edge.from, to: edge.to })
  }

  // ── Ambient particles (background dust) ──
  const particleCount = 200
  const particleGeo = new THREE.BufferGeometry()
  const particlePositions = new Float32Array(particleCount * 3)
  for (let i = 0; i < particleCount; i++) {
    particlePositions[i * 3] = (Math.random() - 0.5) * 300
    particlePositions[i * 3 + 1] = (Math.random() - 0.5) * 200
    particlePositions[i * 3 + 2] = (Math.random() - 0.5) * 300
  }
  particleGeo.setAttribute('position', new THREE.Float32BufferAttribute(particlePositions, 3))
  const particleMat = new THREE.PointsMaterial({
    color: 0x404060,
    size: 0.5,
    transparent: true,
    opacity: 0.4,
    blending: THREE.AdditiveBlending,
    depthWrite: false,
  })
  const particles = new THREE.Points(particleGeo, particleMat)
  scene.add(particles)

  // ── Lighting ──
  const ambientLight = new THREE.AmbientLight(0x303040, 0.6)
  scene.add(ambientLight)

  const pointLight1 = new THREE.PointLight(0x60a5fa, 1.0, 300)
  pointLight1.position.set(50, 80, 50)
  scene.add(pointLight1)

  const pointLight2 = new THREE.PointLight(0xc77d8f, 0.6, 300)
  pointLight2.position.set(-60, -40, -60)
  scene.add(pointLight2)

  const pointLight3 = new THREE.PointLight(0x5eead4, 0.4, 250)
  pointLight3.position.set(0, -50, 80)
  scene.add(pointLight3)

  // ── Raycasting for interaction ──
  const raycaster = new THREE.Raycaster()
  const mouse = new THREE.Vector2()

  let hoveredMesh = null

  function onPointerMove(event) {
    const rect = container.getBoundingClientRect()
    mouse.x = ((event.clientX - rect.left) / rect.width) * 2 - 1
    mouse.y = -((event.clientY - rect.top) / rect.height) * 2 + 1

    raycaster.setFromCamera(mouse, camera)
    const meshList = Object.values(nodeMeshes)
    const intersects = raycaster.intersectObjects(meshList, false)

    // Reset previous hover
    if (hoveredMesh) {
      const prevData = nodeData[hoveredMesh.uuid]
      if (prevData) {
        hoveredMesh.material.emissiveIntensity = 0.5
        hoveredMesh.scale.set(1, 1, 1)
      }
      // Reset edge dimming
      for (const em of edgeMeshes) {
        em.line.material.opacity = 0.25
      }
      // Reset node dimming
      for (const mesh of meshList) {
        mesh.material.opacity = 1.0
        mesh.material.transparent = false
      }
      container.style.cursor = 'grab'
    }

    if (intersects.length > 0) {
      const hit = intersects[0].object
      if (nodeData[hit.uuid]) {
        hoveredMesh = hit
        const data = nodeData[hit.uuid]
        hit.material.emissiveIntensity = 1.2
        hit.scale.set(1.15, 1.15, 1.15)
        container.style.cursor = 'pointer'

        // Highlight connected edges, dim others
        for (const em of edgeMeshes) {
          if (em.from === data.name || em.to === data.name) {
            em.line.material.opacity = 0.9
          } else {
            em.line.material.opacity = 0.06
          }
        }

        // Dim unrelated nodes
        const related = new Set([data.name, ...data.deps, ...getDependents(data.name)])
        for (const mesh of meshList) {
          const nd = nodeData[mesh.uuid]
          if (nd && !related.has(nd.name)) {
            mesh.material.transparent = true
            mesh.material.opacity = 0.15
          }
        }
      } else {
        hoveredMesh = null
      }
    } else {
      hoveredMesh = null
    }
  }

  function onPointerClick(event) {
    // Ignore if dragging
    if (event.type === 'click') {
      const rect = container.getBoundingClientRect()
      mouse.x = ((event.clientX - rect.left) / rect.width) * 2 - 1
      mouse.y = -((event.clientY - rect.top) / rect.height) * 2 + 1

      raycaster.setFromCamera(mouse, camera)
      const meshList = Object.values(nodeMeshes)
      const intersects = raycaster.intersectObjects(meshList, false)

      if (intersects.length > 0) {
        const hit = intersects[0].object
        const data = nodeData[hit.uuid]
        if (data) {
          selectedCrate.value = data
        }
      }
    }
  }

  function onDblClick(event) {
    const rect = container.getBoundingClientRect()
    mouse.x = ((event.clientX - rect.left) / rect.width) * 2 - 1
    mouse.y = -((event.clientY - rect.top) / rect.height) * 2 + 1

    raycaster.setFromCamera(mouse, camera)
    const meshList = Object.values(nodeMeshes)
    const intersects = raycaster.intersectObjects(meshList, false)

    if (intersects.length > 0) {
      const hit = intersects[0].object
      const data = nodeData[hit.uuid]
      if (data && data.chapter) {
        window.location.href = data.chapter
      }
    }
  }

  container.addEventListener('pointermove', onPointerMove)
  container.addEventListener('click', onPointerClick)
  container.addEventListener('dblclick', onDblClick)

  // ── Animation loop ──
  let animFrameId = null
  let time = 0

  function animate() {
    animFrameId = requestAnimationFrame(animate)
    time += 0.005

    controls.update()

    // Gentle particle drift
    const posArr = particles.geometry.attributes.position.array
    for (let i = 0; i < particleCount; i++) {
      posArr[i * 3 + 1] += Math.sin(time + i * 0.1) * 0.01
    }
    particles.geometry.attributes.position.needsUpdate = true

    // Subtle node breathing (scale pulse)
    for (const crate of crateList) {
      const mesh = nodeMeshes[crate.name]
      if (mesh && mesh !== hoveredMesh) {
        const s = 1.0 + Math.sin(time * 2 + mesh.position.x * 0.1) * 0.015
        mesh.scale.set(s, s, s)
      }
    }

    composer.render()
    labelRenderer.render(scene, camera)
  }

  animate()
  ready.value = true

  // ── Resize handling ──
  function onResize() {
    const w = container.clientWidth
    const h = container.clientHeight
    if (w === 0 || h === 0) return

    camera.aspect = w / h
    camera.updateProjectionMatrix()
    renderer.setSize(w, h)
    labelRenderer.setSize(w, h)
    composer.setSize(w, h)
    bloomPass.resolution.set(w, h)
  }

  const resizeObserver = new ResizeObserver(onResize)
  resizeObserver.observe(container)

  // Store cleanup refs
  threeState.value = {
    renderer,
    labelRenderer,
    composer,
    controls,
    scene,
    animFrameId,
    resizeObserver,
    container,
    onPointerMove,
    onPointerClick,
    onDblClick,
    idleTimer,
  }
}

// ─── Lifecycle ───
onMounted(() => {
  // Delay to ensure DOM is rendered
  requestAnimationFrame(() => {
    initThreeScene()
  })
})

onUnmounted(() => {
  const state = threeState.value
  if (!state) return

  cancelAnimationFrame(state.animFrameId)
  clearTimeout(state.idleTimer)

  state.container.removeEventListener('pointermove', state.onPointerMove)
  state.container.removeEventListener('click', state.onPointerClick)
  state.container.removeEventListener('dblclick', state.onDblClick)

  state.resizeObserver.disconnect()
  state.controls.dispose()
  state.renderer.dispose()
  state.composer.dispose()

  // Remove canvases
  if (state.renderer.domElement.parentNode) {
    state.renderer.domElement.parentNode.removeChild(state.renderer.domElement)
  }
  if (state.labelRenderer.domElement.parentNode) {
    state.labelRenderer.domElement.parentNode.removeChild(state.labelRenderer.domElement)
  }

  // Dispose all scene objects
  state.scene.traverse((obj) => {
    if (obj.geometry) obj.geometry.dispose()
    if (obj.material) {
      if (Array.isArray(obj.material)) {
        obj.material.forEach(m => m.dispose())
      } else {
        obj.material.dispose()
      }
    }
  })

  threeState.value = null
})

// Keyboard: Escape to deselect
function onKeyDown(e) {
  if (e.key === 'Escape') selectedCrate.value = null
}

onMounted(() => document.addEventListener('keydown', onKeyDown))
onUnmounted(() => document.removeEventListener('keydown', onKeyDown))
</script>

<style scoped>
.crate-graph-3d {
  position: relative;
  width: 100%;
  height: 600px;
  border-radius: 12px;
  overflow: hidden;
  margin: 1.5rem 0;
  background: #0a0a0f;
  border: 1px solid #1e1e2e;
}

.cg3d-canvas {
  width: 100%;
  height: 100%;
  cursor: grab;
}

.cg3d-canvas:active {
  cursor: grabbing;
}

.cg3d-labels {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
  overflow: hidden;
}

/* ── Loading ── */
.cg3d-loading {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  background: #0a0a0f;
  color: #6b7280;
  font-size: 14px;
  z-index: 20;
}

.cg3d-spinner {
  width: 32px;
  height: 32px;
  border: 3px solid #1e1e2e;
  border-top-color: #60a5fa;
  border-radius: 50%;
  animation: cg3d-spin 0.8s linear infinite;
}

@keyframes cg3d-spin {
  to { transform: rotate(360deg); }
}

/* ── Legend ── */
.cg3d-legend {
  position: absolute;
  top: 12px;
  left: 12px;
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
  z-index: 10;
  pointer-events: none;
}

.cg3d-legend-item {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 11px;
  font-weight: 500;
  color: rgba(255, 255, 255, 0.55);
  text-shadow: 0 1px 3px rgba(0, 0, 0, 0.8);
  letter-spacing: 0.03em;
}

.cg3d-legend-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
  box-shadow: 0 0 6px currentColor;
}

/* ── Hint ── */
.cg3d-hint {
  position: absolute;
  bottom: 12px;
  left: 50%;
  transform: translateX(-50%);
  font-size: 12px;
  color: rgba(255, 255, 255, 0.35);
  background: rgba(10, 10, 15, 0.7);
  padding: 6px 14px;
  border-radius: 20px;
  pointer-events: none;
  z-index: 10;
  white-space: nowrap;
  backdrop-filter: blur(4px);
  border: 1px solid rgba(255, 255, 255, 0.06);
}

/* ── Info Panel ── */
.cg3d-panel {
  position: absolute;
  top: 12px;
  right: 12px;
  width: 280px;
  max-height: calc(100% - 24px);
  overflow-y: auto;
  background: rgba(15, 15, 25, 0.92);
  backdrop-filter: blur(12px);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 10px;
  padding: 16px;
  z-index: 15;
  color: #e2e8f0;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
}

.cg3d-panel-close {
  position: absolute;
  top: 8px;
  right: 10px;
  background: none;
  border: none;
  font-size: 20px;
  cursor: pointer;
  color: #6b7280;
  line-height: 1;
  padding: 4px;
}

.cg3d-panel-close:hover {
  color: #e2e8f0;
}

.cg3d-panel-name {
  margin: 0 0 6px;
  font-size: 15px;
  font-weight: 700;
  color: #f1f5f9;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, monospace;
}

.cg3d-panel-desc {
  margin: 0 0 12px;
  font-size: 12px;
  line-height: 1.5;
  color: #94a3b8;
}

.cg3d-panel-stats {
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
  gap: 6px;
  margin-bottom: 12px;
}

.cg3d-stat {
  text-align: center;
  padding: 8px 4px;
  background: rgba(255, 255, 255, 0.04);
  border-radius: 6px;
  border: 1px solid rgba(255, 255, 255, 0.05);
}

.cg3d-stat-val {
  display: block;
  font-size: 15px;
  font-weight: 700;
  color: #f1f5f9;
}

.cg3d-stat-lbl {
  display: block;
  font-size: 9px;
  color: #6b7280;
  margin-top: 2px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.cg3d-panel-link {
  margin-bottom: 12px;
}

.cg3d-panel-link a {
  display: inline-block;
  padding: 5px 12px;
  background: rgba(96, 165, 250, 0.15);
  color: #60a5fa;
  border: 1px solid rgba(96, 165, 250, 0.25);
  border-radius: 5px;
  font-size: 12px;
  font-weight: 500;
  text-decoration: none;
  transition: background 0.15s, border-color 0.15s;
}

.cg3d-panel-link a:hover {
  background: rgba(96, 165, 250, 0.25);
  border-color: rgba(96, 165, 250, 0.4);
}

.cg3d-panel-deps {
  margin-bottom: 10px;
}

.cg3d-panel-deps h4 {
  margin: 0 0 6px;
  font-size: 10px;
  font-weight: 600;
  color: #6b7280;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.cg3d-dep-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.cg3d-dep-tag {
  display: inline-block;
  padding: 2px 8px;
  border: 1.5px solid #475569;
  border-radius: 12px;
  font-size: 10px;
  font-weight: 500;
  color: #cbd5e1;
  cursor: pointer;
  transition: background 0.15s;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, monospace;
}

.cg3d-dep-tag:hover {
  background: rgba(255, 255, 255, 0.06);
}

.cg3d-dep-tag--dependent {
  border-style: dashed;
}

/* ── Panel transitions ── */
.cg3d-panel-enter-active,
.cg3d-panel-leave-active {
  transition: transform 0.2s ease, opacity 0.2s ease;
}

.cg3d-panel-enter-from,
.cg3d-panel-leave-to {
  transform: translateX(20px);
  opacity: 0;
}

/* ── Scrollbar styling for panel ── */
.cg3d-panel::-webkit-scrollbar {
  width: 4px;
}

.cg3d-panel::-webkit-scrollbar-track {
  background: transparent;
}

.cg3d-panel::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.1);
  border-radius: 2px;
}

/* ── Responsive ── */
@media (max-width: 768px) {
  .crate-graph-3d {
    height: 400px;
  }

  .cg3d-panel {
    width: 240px;
    padding: 12px;
  }

  .cg3d-panel-stats {
    grid-template-columns: 1fr 1fr;
  }

  .cg3d-legend {
    gap: 8px;
  }

  .cg3d-legend-item {
    font-size: 10px;
  }
}

@media (max-width: 480px) {
  .crate-graph-3d {
    height: 320px;
  }

  .cg3d-panel {
    top: auto;
    bottom: 0;
    right: 0;
    left: 0;
    width: 100%;
    max-height: 50%;
    border-radius: 10px 10px 0 0;
  }
}
</style>

<style>
/* ── Global styles for CSS2D labels (must not be scoped) ── */
.cg3d-node-label {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, monospace;
  font-size: 10px;
  font-weight: 600;
  text-shadow: 0 0 8px rgba(0, 0, 0, 0.9), 0 0 2px rgba(0, 0, 0, 0.7);
  pointer-events: none;
  user-select: none;
  white-space: nowrap;
  letter-spacing: 0.02em;
  opacity: 0.85;
}
</style>
