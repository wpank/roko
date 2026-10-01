<template>
  <div class="arch-explorer" ref="container">
    <div class="arch-toolbar">
      <button @click="zoomIn" title="Zoom in">+</button>
      <button @click="zoomOut" title="Zoom out">&minus;</button>
      <button @click="resetView" title="Reset view">Reset</button>
      <span class="arch-legend">
        <span v-for="tier in tiers" :key="tier.id" class="legend-item">
          <span class="legend-dot" :style="{ background: tier.color }"></span>
          {{ tier.label }}
        </span>
      </span>
    </div>

    <svg
      ref="svg"
      :viewBox="viewBox"
      class="arch-svg"
      @mousedown="onPanStart"
      @mousemove="onPanMove"
      @mouseup="onPanEnd"
      @mouseleave="onPanEnd"
      @wheel.prevent="onWheel"
    >
      <defs>
        <marker
          id="arrowhead"
          markerWidth="8"
          markerHeight="6"
          refX="8"
          refY="3"
          orient="auto"
        >
          <polygon points="0 0, 8 3, 0 6" fill="#6b7280" />
        </marker>
        <marker
          id="arrowhead-highlight"
          markerWidth="8"
          markerHeight="6"
          refX="8"
          refY="3"
          orient="auto"
        >
          <polygon points="0 0, 8 3, 0 6" fill="#f59e0b" />
        </marker>
        <filter id="node-shadow" x="-20%" y="-20%" width="140%" height="140%">
          <feDropShadow dx="1" dy="2" stdDeviation="2" flood-opacity="0.15" />
        </filter>
        <filter id="node-glow" x="-30%" y="-30%" width="160%" height="160%">
          <feGaussianBlur in="SourceAlpha" stdDeviation="4" result="blur" />
          <feFlood flood-color="#f59e0b" flood-opacity="0.35" result="color" />
          <feComposite in="color" in2="blur" operator="in" result="glow" />
          <feMerge>
            <feMergeNode in="glow" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
      </defs>

      <g :transform="transformStr">
        <!-- Tier background bands -->
        <rect
          v-for="band in tierBands"
          :key="'band-' + band.tier"
          :x="band.x"
          :y="band.y"
          :width="band.width"
          :height="band.height"
          :fill="band.fill"
          rx="8"
        />
        <text
          v-for="band in tierBands"
          :key="'band-label-' + band.tier"
          :x="band.x + 12"
          :y="band.y + 20"
          class="tier-label"
          :fill="band.labelColor"
        >{{ band.label }}</text>

        <!-- Edges -->
        <path
          v-for="edge in edges"
          :key="edge.id"
          :d="edge.path"
          class="edge-line"
          :class="{
            'edge-highlighted': isEdgeHighlighted(edge),
            'edge-dimmed': hoveredCrate && !isEdgeHighlighted(edge),
          }"
          :marker-end="isEdgeHighlighted(edge) ? 'url(#arrowhead-highlight)' : 'url(#arrowhead)'"
        />

        <!-- Nodes -->
        <g
          v-for="node in nodes"
          :key="node.id"
          :transform="`translate(${node.x}, ${node.y})`"
          class="crate-node"
          :class="{
            'node-selected': selectedCrate === node.id,
            'node-hovered': hoveredCrate === node.id,
            'node-related': isRelated(node.id),
            'node-dimmed': hoveredCrate && hoveredCrate !== node.id && !isRelated(node.id),
          }"
          @click.stop="selectCrate(node.id)"
          @mouseenter="hoveredCrate = node.id"
          @mouseleave="hoveredCrate = null"
          :filter="selectedCrate === node.id ? 'url(#node-glow)' : 'url(#node-shadow)'"
        >
          <rect
            :width="node.width"
            :height="node.height"
            :rx="6"
            :fill="node.color"
            class="node-rect"
          />
          <text
            :x="node.width / 2"
            :y="22"
            text-anchor="middle"
            class="node-name"
          >{{ node.shortName }}</text>
          <text
            :x="node.width / 2"
            :y="38"
            text-anchor="middle"
            class="node-stats"
          >{{ formatLoc(node.loc) }} LOC &middot; {{ node.tests }} tests</text>
        </g>
      </g>
    </svg>

    <!-- Detail panel -->
    <transition name="panel-slide">
      <div v-if="selectedCrate" class="detail-panel" @click.stop>
        <button class="panel-close" @click="selectedCrate = null">&times;</button>
        <h3>{{ crateData[selectedCrate].name }}</h3>
        <p class="panel-desc">{{ crateData[selectedCrate].description }}</p>

        <div class="panel-stats">
          <div class="stat">
            <span class="stat-value">{{ formatLoc(crateData[selectedCrate].loc) }}</span>
            <span class="stat-label">Lines of Code</span>
          </div>
          <div class="stat">
            <span class="stat-value">{{ crateData[selectedCrate].tests }}</span>
            <span class="stat-label">Tests</span>
          </div>
          <div class="stat">
            <span class="stat-value">{{ tierLabel(crateData[selectedCrate].tier) }}</span>
            <span class="stat-label">Tier</span>
          </div>
        </div>

        <div v-if="crateData[selectedCrate].chapter" class="panel-link">
          <a :href="crateData[selectedCrate].chapter">
            View documentation &rarr;
          </a>
        </div>

        <div class="panel-deps" v-if="getDepNames(selectedCrate).length">
          <h4>Dependencies ({{ getDepNames(selectedCrate).length }})</h4>
          <div class="dep-tags">
            <span
              v-for="dep in getDepNames(selectedCrate)"
              :key="dep"
              class="dep-tag"
              :style="{ borderColor: getCrateColor(dep) }"
              @click="selectCrate(dep)"
            >{{ dep }}</span>
          </div>
        </div>

        <div class="panel-deps" v-if="getDependentNames(selectedCrate).length">
          <h4>Dependents ({{ getDependentNames(selectedCrate).length }})</h4>
          <div class="dep-tags">
            <span
              v-for="dep in getDependentNames(selectedCrate)"
              :key="dep"
              class="dep-tag dependent-tag"
              :style="{ borderColor: getCrateColor(dep) }"
              @click="selectCrate(dep)"
            >{{ dep }}</span>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'

// --- Tier definitions ---
const tiers = [
  { id: 'primitives', label: 'Primitives',  color: '#6366f1' },
  { id: 'kernel',     label: 'Kernel',      color: '#3b82f6' },
  { id: 'engine',     label: 'Engine',      color: '#22c55e' },
  { id: 'agent',      label: 'Agent',       color: '#f97316' },
  { id: 'cognitive',  label: 'Cognitive',   color: '#a855f7' },
  { id: 'surface',    label: 'Surface',     color: '#ec4899' },
  { id: 'infra',      label: 'Infra',       color: '#64748b' },
]

const tierColors = Object.fromEntries(tiers.map(t => [t.id, t.color]))

// --- Crate data (static, derived from workspace analysis) ---
const crateData = {
  'roko-primitives':    { name: 'roko-primitives',    tier: 'primitives', loc: 5241,   tests: 142,  description: 'HDC vectors, tier routing, and shared mathematical primitives.',              chapter: './01-SIGNAL.html', deps: [] },
  'roko-core':          { name: 'roko-core',          tier: 'kernel',     loc: 92820,  tests: 1849, description: 'Signal + 12 kernel traits (Store, Score, Route, Compose, React, Bus, Observe, Connect, Trigger, Verify, Substrate, ColdStore). Config, tools, errors.', chapter: './01-SIGNAL.html', deps: ['roko-primitives'] },
  'roko-fs':            { name: 'roko-fs',            tier: 'kernel',     loc: 11848,  tests: 158,  description: 'Filesystem-backed Substrate: JSONL signal log, GC, layout conventions.',       chapter: './01-SIGNAL.html', deps: ['roko-core'] },
  'roko-std':           { name: 'roko-std',           tier: 'kernel',     loc: 9337,   tests: 218,  description: '35 standard tool definitions (16 local + 19 GitHub MCP). Simple router/scorer impls.', chapter: './19-TOOLS-PLUGINS.html', deps: ['roko-core'] },
  'roko-eval':          { name: 'roko-eval',          tier: 'kernel',     loc: 2400,   tests: 45,   description: 'Unified evaluation framework: EvidenceCollector, Criterion, Profile traits.',   chapter: './07-GATES.html', deps: ['roko-core'] },
  'roko-graph':         { name: 'roko-graph',         tier: 'engine',     loc: 25686,  tests: 400,  description: 'Sole execution engine: DAG cells, topology, cost state, bounded parallel waves, conditional routing, immune Graph.', chapter: './03-GRAPH.html', deps: ['roko-core'] },
  'roko-gate':          { name: 'roko-gate',          tier: 'engine',     loc: 28704,  tests: 546,  description: '19 gates, 7-rung pipeline, adaptive thresholds. Compile, test, clippy, diff, and custom oracle gates.', chapter: './07-GATES.html', deps: ['roko-core', 'roko-agent', 'roko-std'] },
  'roko-plugin':        { name: 'roko-plugin',        tier: 'engine',     loc: 5665,   tests: 92,   description: 'Plugin SDK: signed manifests, WASM hooks, dependency resolution, strict admission, capability policy.', chapter: './19-TOOLS-PLUGINS.html', deps: ['roko-core'] },
  'roko-agent':         { name: 'roko-agent',         tier: 'agent',      loc: 107778, tests: 1721, description: '12 LLM provider kinds (Anthropic, Claude CLI, Codex, OpenAI, Cursor, Perplexity, Gemini, Cerebras, Hermes, OpenClaw). Pools, MCP, tool loop, safety.', chapter: './05-AGENT.html', deps: ['roko-core', 'roko-fs', 'roko-graph', 'roko-std'] },
  'roko-daimon':        { name: 'roko-daimon',        tier: 'cognitive',  loc: 8446,   tests: 104,  description: 'Affect engine: somatic markers, emotional state, dispatch modulation. CorticalState energy fields.', chapter: './11-AFFECT.html', deps: ['roko-core'] },
  'roko-learn':         { name: 'roko-learn',         tier: 'cognitive',  loc: 72888,  tests: 954,  description: 'Episodes, playbooks, bandits, model routing, A/B experiments, efficiency tracking, HDC consolidation, hindsight.', chapter: './08-LEARNING.html', deps: ['roko-core', 'roko-agent', 'roko-daimon', 'roko-fs', 'roko-primitives'] },
  'roko-neuro':         { name: 'roko-neuro',         tier: 'cognitive',  loc: 22365,  tests: 243,  description: 'Durable knowledge store: tier progression, distillation, HDC fingerprinting, temporal queries.', chapter: './09-MEMORY.html', deps: ['roko-core', 'roko-fs', 'roko-agent', 'roko-learn'] },
  'roko-dreams':        { name: 'roko-dreams',        tier: 'cognitive',  loc: 14545,  tests: 89,   description: 'Offline consolidation: hypnagogia, imagination cycles, dream journals, adaptive scheduling.', chapter: './10-DREAMS.html', deps: ['roko-core', 'roko-neuro', 'roko-learn', 'roko-agent', 'roko-primitives'] },
  'roko-compose':       { name: 'roko-compose',       tier: 'engine',     loc: 32500,  tests: 419,  description: 'Prompt assembly: 11 role templates, 9-layer SystemPromptBuilder, enrichment, attention bidders.', chapter: './06-COMPOSITION.html', deps: ['roko-agent', 'roko-core', 'roko-daimon', 'roko-dreams', 'roko-graph', 'roko-learn', 'roko-neuro', 'roko-std'] },
  'roko-conductor':     { name: 'roko-conductor',     tier: 'engine',     loc: 10665,  tests: 320,  description: '12 watchers, circuit breaker, diagnosis. Reactive intelligence for plan execution.', chapter: './30-CONDUCTOR.html', deps: ['roko-core', 'roko-learn'] },
  'roko-gateway':       { name: 'roko-gateway',       tier: 'engine',     loc: 4858,   tests: 26,   description: 'Nine-stage inference gateway: routing, caching, cost accounting, backpressure, key rotation.', chapter: './20-GATEWAY.html', deps: ['roko-core', 'roko-agent', 'roko-learn', 'roko-graph'] },
  'roko-runtime':       { name: 'roko-runtime',       tier: 'engine',     loc: 28535,  tests: 285,  description: 'ProcessSupervisor, event bus, cancellation tokens, workflow contracts. Agent lifecycle management.', chapter: './04-EXECUTION.html', deps: ['roko-primitives', 'roko-core', 'roko-compose', 'roko-learn', 'roko-gate'] },
  'roko-execution':     { name: 'roko-execution',     tier: 'engine',     loc: 12036,  tests: 239,  description: 'RuntimeServices builder: shared service facade for CLI/serve/ACP. Safety, budget, routing, feedback.', chapter: './04-EXECUTION.html', deps: ['roko-core', 'roko-agent', 'roko-gate', 'roko-learn', 'roko-compose', 'roko-graph', 'roko-runtime', 'roko-neuro', 'roko-fs'] },
  'roko-chain':         { name: 'roko-chain',         tier: 'infra',      loc: 29683,  tests: 340,  description: 'Optional chain client: identity, delegation, knowledge registry, marketplace, arena, DeFi state machines.', chapter: './23-PAYMENTS-ECONOMY.html', deps: ['roko-core'] },
  'roko-index':         { name: 'roko-index',         tier: 'infra',      loc: 5822,   tests: 88,   description: 'Code intelligence: parser, symbol graph, PageRank, HDC fingerprinting for codebase understanding.', chapter: './34-CODE-INTELLIGENCE.html', deps: ['roko-core', 'roko-primitives'] },
  'roko-agent-server':  { name: 'roko-agent-server',  tier: 'surface',    loc: 6308,   tests: 21,   description: 'Per-agent HTTP sidecar: /message (real LLM dispatch), /stream WS, /predictions, /research.', chapter: './27-ACP.html', deps: ['roko-agent', 'roko-core', 'roko-fs', 'roko-learn', 'roko-neuro'] },
  'roko-serve':         { name: 'roko-serve',         tier: 'surface',    loc: 101157, tests: 467,  description: 'HTTP control plane: REST routes + SSE + WebSocket on :6677. Dashboard, telemetry, marketplace.', chapter: './26-HTTP-API.html', deps: ['roko-core', 'roko-agent', 'roko-agent-server', 'roko-learn', 'roko-neuro', 'roko-dreams', 'roko-gate', 'roko-gateway', 'roko-fs', 'roko-compose', 'roko-std', 'roko-plugin', 'roko-daimon', 'roko-runtime', 'roko-execution'] },
  'roko-acp':           { name: 'roko-acp',           tier: 'surface',    loc: 20817,  tests: 120,  description: 'Agent Client Protocol server for Cursor/external editor integration. 180 ACP tests, 8/8 wired.', chapter: './27-ACP.html', deps: ['roko-core', 'roko-execution', 'roko-runtime', 'roko-agent', 'roko-gate', 'roko-compose', 'roko-serve', 'roko-learn', 'roko-dreams', 'roko-neuro'] },
  'roko-cli':           { name: 'roko-cli',           tier: 'surface',    loc: 264660, tests: 3109, description: 'Main binary: plan DAG/runner, merge queue, worktree manager, ratatui TUI, all CLI subcommands.', chapter: './28-CLI.html', deps: ['roko-core', 'roko-std', 'roko-fs', 'roko-learn', 'roko-compose', 'roko-agent', 'roko-agent-server', 'roko-gate', 'roko-dreams', 'roko-daimon', 'roko-neuro', 'roko-conductor', 'roko-plugin', 'roko-runtime', 'roko-serve'] },
}

// --- Layout computation ---
// Arrange crates in tier rows with manual positioning for clarity
const layout = {
  width: 1400,
  height: 900,
  nodeWidth: 140,
  nodeHeight: 48,
  tierRows: {
    primitives: { y: 40,  crates: ['roko-primitives'] },
    kernel:     { y: 140, crates: ['roko-core', 'roko-fs', 'roko-std', 'roko-eval'] },
    engine:     { y: 280, crates: ['roko-graph', 'roko-gate', 'roko-compose', 'roko-plugin', 'roko-conductor', 'roko-gateway', 'roko-runtime', 'roko-execution'] },
    agent:      { y: 440, crates: ['roko-agent'] },
    cognitive:  { y: 560, crates: ['roko-daimon', 'roko-learn', 'roko-neuro', 'roko-dreams'] },
    surface:    { y: 700, crates: ['roko-cli', 'roko-serve', 'roko-acp', 'roko-agent-server'] },
    infra:      { y: 830, crates: ['roko-chain', 'roko-index'] },
  },
}

function computeNodePositions() {
  const positions = {}
  for (const [tierId, tierRow] of Object.entries(layout.tierRows)) {
    const count = tierRow.crates.length
    const totalWidth = count * layout.nodeWidth + (count - 1) * 24
    const startX = (layout.width - totalWidth) / 2
    tierRow.crates.forEach((crateId, i) => {
      positions[crateId] = {
        x: startX + i * (layout.nodeWidth + 24),
        y: tierRow.y,
      }
    })
  }
  return positions
}

const nodePositions = computeNodePositions()

const nodes = computed(() => {
  return Object.entries(crateData).map(([id, data]) => {
    const pos = nodePositions[id] || { x: 0, y: 0 }
    return {
      id,
      x: pos.x,
      y: pos.y,
      width: layout.nodeWidth,
      height: layout.nodeHeight,
      color: tierColors[data.tier] || '#64748b',
      shortName: id.replace('roko-', ''),
      loc: data.loc,
      tests: data.tests,
    }
  })
})

// Tier background bands
const tierBands = computed(() => {
  return tiers.map(tier => {
    const row = layout.tierRows[tier.id]
    if (!row) return null
    const crateIds = row.crates
    const positions = crateIds.map(c => nodePositions[c]).filter(Boolean)
    if (!positions.length) return null
    const minX = Math.min(...positions.map(p => p.x)) - 16
    const maxX = Math.max(...positions.map(p => p.x)) + layout.nodeWidth + 16
    return {
      tier: tier.id,
      x: minX,
      y: row.y - 16,
      width: maxX - minX,
      height: layout.nodeHeight + 32,
      fill: tier.color + '0a',
      labelColor: tier.color + '80',
      label: tier.label,
    }
  }).filter(Boolean)
})

// Edge computation with curved paths
const edges = computed(() => {
  const result = []
  for (const [id, data] of Object.entries(crateData)) {
    for (const dep of data.deps) {
      if (!crateData[dep]) continue
      const fromPos = nodePositions[id]
      const toPos = nodePositions[dep]
      if (!fromPos || !toPos) continue

      const fromX = fromPos.x + layout.nodeWidth / 2
      const fromY = fromPos.y
      const toX = toPos.x + layout.nodeWidth / 2
      const toY = toPos.y + layout.nodeHeight

      // Curved path: bezier from bottom of dependency to top of dependent
      const midY = (fromY + toY) / 2
      const path = `M ${toX} ${toY} C ${toX} ${midY}, ${fromX} ${midY}, ${fromX} ${fromY}`

      result.push({
        id: `${dep}->${id}`,
        from: dep,
        to: id,
        path,
      })
    }
  }
  return result
})

// --- Interaction state ---
const selectedCrate = ref(null)
const hoveredCrate = ref(null)

// Pan and zoom state
const scale = ref(1)
const panX = ref(0)
const panY = ref(0)
const isPanning = ref(false)
const panStartX = ref(0)
const panStartY = ref(0)
const panStartPanX = ref(0)
const panStartPanY = ref(0)

const container = ref(null)
const svg = ref(null)

const viewBox = computed(() => `0 0 ${layout.width} ${layout.height}`)

const transformStr = computed(() => {
  return `translate(${panX.value}, ${panY.value}) scale(${scale.value})`
})

function zoomIn() {
  scale.value = Math.min(scale.value * 1.2, 3)
}

function zoomOut() {
  scale.value = Math.max(scale.value / 1.2, 0.3)
}

function resetView() {
  scale.value = 1
  panX.value = 0
  panY.value = 0
}

function onWheel(e) {
  const delta = e.deltaY > 0 ? 0.9 : 1.1
  scale.value = Math.max(0.3, Math.min(3, scale.value * delta))
}

function onPanStart(e) {
  if (e.target.closest('.crate-node')) return
  isPanning.value = true
  panStartX.value = e.clientX
  panStartY.value = e.clientY
  panStartPanX.value = panX.value
  panStartPanY.value = panY.value
}

function onPanMove(e) {
  if (!isPanning.value) return
  const dx = e.clientX - panStartX.value
  const dy = e.clientY - panStartY.value
  panX.value = panStartPanX.value + dx / scale.value
  panY.value = panStartPanY.value + dy / scale.value
}

function onPanEnd() {
  isPanning.value = false
}

// --- Selection / hover helpers ---
function selectCrate(id) {
  selectedCrate.value = selectedCrate.value === id ? null : id
}

function isRelated(id) {
  if (!hoveredCrate.value) return false
  const hovered = crateData[hoveredCrate.value]
  if (!hovered) return false
  // Direct dependency
  if (hovered.deps.includes(id)) return true
  // Direct dependent
  const target = crateData[id]
  if (target && target.deps.includes(hoveredCrate.value)) return true
  return false
}

function isEdgeHighlighted(edge) {
  if (!hoveredCrate.value) return false
  return edge.from === hoveredCrate.value || edge.to === hoveredCrate.value
}

function getDepNames(id) {
  const data = crateData[id]
  return data ? data.deps.filter(d => crateData[d]) : []
}

function getDependentNames(id) {
  return Object.entries(crateData)
    .filter(([, data]) => data.deps.includes(id))
    .map(([depId]) => depId)
}

function getCrateColor(id) {
  const data = crateData[id]
  return data ? tierColors[data.tier] || '#64748b' : '#64748b'
}

function tierLabel(tierId) {
  const tier = tiers.find(t => t.id === tierId)
  return tier ? tier.label : tierId
}

function formatLoc(n) {
  if (n >= 1000) return (n / 1000).toFixed(1) + 'k'
  return String(n)
}

// Keyboard navigation
function onKeyDown(e) {
  if (e.key === 'Escape') {
    selectedCrate.value = null
  }
}

onMounted(() => {
  document.addEventListener('keydown', onKeyDown)
})

onUnmounted(() => {
  document.removeEventListener('keydown', onKeyDown)
})
</script>

<style scoped>
.arch-explorer {
  position: relative;
  width: 100%;
  border: 1px solid var(--vp-c-divider, #e2e8f0);
  border-radius: 8px;
  overflow: hidden;
  background: var(--vp-c-bg, #ffffff);
  margin: 1.5rem 0;
}

.arch-toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--vp-c-divider, #e2e8f0);
  background: var(--vp-c-bg-soft, #f8fafc);
  flex-wrap: wrap;
}

.arch-toolbar button {
  padding: 4px 12px;
  border: 1px solid var(--vp-c-divider, #e2e8f0);
  border-radius: 4px;
  background: var(--vp-c-bg, #ffffff);
  color: var(--vp-c-text-1, #1e293b);
  cursor: pointer;
  font-size: 14px;
  font-weight: 500;
  transition: background 0.15s;
}

.arch-toolbar button:hover {
  background: var(--vp-c-bg-elv, #f1f5f9);
}

.arch-legend {
  display: flex;
  gap: 12px;
  margin-left: auto;
  flex-wrap: wrap;
}

.legend-item {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  color: var(--vp-c-text-2, #64748b);
}

.legend-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.arch-svg {
  width: 100%;
  height: 600px;
  cursor: grab;
  user-select: none;
}

.arch-svg:active {
  cursor: grabbing;
}

/* Tier labels */
.tier-label {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

/* Edges */
.edge-line {
  fill: none;
  stroke: #cbd5e1;
  stroke-width: 1.2;
  transition: stroke 0.2s, stroke-width 0.2s, opacity 0.2s;
}

.edge-highlighted {
  stroke: #f59e0b;
  stroke-width: 2.2;
  opacity: 1 !important;
}

.edge-dimmed {
  opacity: 0.12;
}

/* Nodes */
.crate-node {
  cursor: pointer;
  transition: opacity 0.2s;
}

.crate-node .node-rect {
  stroke: transparent;
  stroke-width: 2;
  transition: stroke 0.15s, fill-opacity 0.15s;
}

.crate-node:hover .node-rect {
  stroke: #f59e0b;
}

.node-selected .node-rect {
  stroke: #f59e0b;
  stroke-width: 2.5;
}

.node-dimmed {
  opacity: 0.25;
}

.node-related {
  opacity: 1;
}

.node-name {
  fill: #ffffff;
  font-size: 12px;
  font-weight: 600;
  pointer-events: none;
}

.node-stats {
  fill: rgba(255, 255, 255, 0.75);
  font-size: 9px;
  pointer-events: none;
}

/* Detail panel */
.detail-panel {
  position: absolute;
  top: 56px;
  right: 12px;
  width: 300px;
  max-height: calc(100% - 68px);
  overflow-y: auto;
  background: var(--vp-c-bg, #ffffff);
  border: 1px solid var(--vp-c-divider, #e2e8f0);
  border-radius: 8px;
  padding: 16px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.1);
  z-index: 10;
}

.panel-close {
  position: absolute;
  top: 8px;
  right: 8px;
  background: none;
  border: none;
  font-size: 20px;
  cursor: pointer;
  color: var(--vp-c-text-2, #64748b);
  line-height: 1;
  padding: 4px;
}

.panel-close:hover {
  color: var(--vp-c-text-1, #1e293b);
}

.detail-panel h3 {
  margin: 0 0 8px;
  font-size: 16px;
  font-weight: 700;
  color: var(--vp-c-text-1, #1e293b);
}

.panel-desc {
  margin: 0 0 12px;
  font-size: 13px;
  line-height: 1.5;
  color: var(--vp-c-text-2, #64748b);
}

.panel-stats {
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
  gap: 8px;
  margin-bottom: 12px;
}

.stat {
  text-align: center;
  padding: 8px 4px;
  background: var(--vp-c-bg-soft, #f8fafc);
  border-radius: 6px;
}

.stat-value {
  display: block;
  font-size: 16px;
  font-weight: 700;
  color: var(--vp-c-text-1, #1e293b);
}

.stat-label {
  display: block;
  font-size: 10px;
  color: var(--vp-c-text-3, #94a3b8);
  margin-top: 2px;
}

.panel-link {
  margin-bottom: 12px;
}

.panel-link a {
  display: inline-block;
  padding: 6px 12px;
  background: var(--vp-c-brand-1, #3b82f6);
  color: #ffffff;
  border-radius: 4px;
  font-size: 13px;
  font-weight: 500;
  text-decoration: none;
  transition: background 0.15s;
}

.panel-link a:hover {
  background: var(--vp-c-brand-2, #2563eb);
}

.panel-deps {
  margin-bottom: 12px;
}

.panel-deps h4 {
  margin: 0 0 6px;
  font-size: 12px;
  font-weight: 600;
  color: var(--vp-c-text-2, #64748b);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.dep-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.dep-tag {
  display: inline-block;
  padding: 2px 8px;
  border: 1.5px solid #cbd5e1;
  border-radius: 12px;
  font-size: 11px;
  font-weight: 500;
  color: var(--vp-c-text-1, #1e293b);
  cursor: pointer;
  transition: background 0.15s;
}

.dep-tag:hover {
  background: var(--vp-c-bg-soft, #f8fafc);
}

.dependent-tag {
  border-style: dashed;
}

/* Panel transition */
.panel-slide-enter-active,
.panel-slide-leave-active {
  transition: transform 0.2s ease, opacity 0.2s ease;
}

.panel-slide-enter-from,
.panel-slide-leave-to {
  transform: translateX(20px);
  opacity: 0;
}

/* Dark mode overrides */
.dark .arch-explorer {
  border-color: #334155;
}

.dark .arch-toolbar {
  background: #1e293b;
  border-color: #334155;
}

.dark .arch-toolbar button {
  background: #0f172a;
  border-color: #334155;
  color: #e2e8f0;
}

.dark .arch-toolbar button:hover {
  background: #1e293b;
}

.dark .edge-line {
  stroke: #475569;
}

.dark .detail-panel {
  background: #1e293b;
  border-color: #334155;
}

.dark .detail-panel h3 {
  color: #f1f5f9;
}

.dark .stat {
  background: #0f172a;
}

.dark .stat-value {
  color: #f1f5f9;
}

.dark .dep-tag {
  color: #e2e8f0;
}

.dark .dep-tag:hover {
  background: #334155;
}
</style>
