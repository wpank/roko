<template>
  <div class="flow-animator">
    <div class="flow-header">
      <h3 class="flow-title">Task Pipeline: Signal Flow</h3>
      <div class="flow-controls">
        <button @click="togglePlay" :title="isPlaying ? 'Pause' : 'Play'">
          {{ isPlaying ? 'Pause' : 'Play' }}
        </button>
        <button @click="stepForward" :disabled="isPlaying" title="Step forward">
          Step &rarr;
        </button>
        <button @click="reset" title="Reset">
          Reset
        </button>
        <span class="flow-step-label">
          Step {{ currentStep + 1 }} / {{ stages.length }}
        </span>
      </div>
    </div>

    <svg :viewBox="`0 0 ${svgWidth} ${svgHeight}`" class="flow-svg">
      <defs>
        <marker
          id="flow-arrow"
          markerWidth="8"
          markerHeight="6"
          refX="8"
          refY="3"
          orient="auto"
        >
          <polygon points="0 0, 8 3, 0 6" fill="#94a3b8" />
        </marker>
        <marker
          id="flow-arrow-active"
          markerWidth="8"
          markerHeight="6"
          refX="8"
          refY="3"
          orient="auto"
        >
          <polygon points="0 0, 8 3, 0 6" fill="#3b82f6" />
        </marker>

        <!-- Animated dot gradient -->
        <radialGradient id="dot-gradient">
          <stop offset="0%" stop-color="#f59e0b" />
          <stop offset="100%" stop-color="#f97316" />
        </radialGradient>

        <!-- Glow filter for active nodes -->
        <filter id="active-glow" x="-30%" y="-30%" width="160%" height="160%">
          <feGaussianBlur in="SourceAlpha" stdDeviation="3" result="blur" />
          <feFlood flood-color="#3b82f6" flood-opacity="0.3" result="color" />
          <feComposite in="color" in2="blur" operator="in" result="glow" />
          <feMerge>
            <feMergeNode in="glow" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>

        <filter id="dot-glow" x="-100%" y="-100%" width="300%" height="300%">
          <feGaussianBlur in="SourceGraphic" stdDeviation="3" result="blur" />
          <feMerge>
            <feMergeNode in="blur" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
      </defs>

      <!-- Connection lines -->
      <line
        v-for="(conn, i) in connections"
        :key="'conn-' + i"
        :x1="conn.x1"
        :y1="conn.y1"
        :x2="conn.x2"
        :y2="conn.y2"
        class="flow-connection"
        :class="{ 'connection-active': isConnectionActive(i) }"
        :marker-end="isConnectionActive(i) ? 'url(#flow-arrow-active)' : 'url(#flow-arrow)'"
      />

      <!-- Stage nodes -->
      <g
        v-for="(stage, i) in stagePositions"
        :key="'stage-' + i"
        :transform="`translate(${stage.x}, ${stage.y})`"
        class="flow-stage"
        :class="{
          'stage-active': i === currentStep,
          'stage-completed': i < currentStep,
          'stage-pending': i > currentStep,
        }"
        @click="jumpToStep(i)"
        :filter="i === currentStep ? 'url(#active-glow)' : undefined"
      >
        <!-- Node box -->
        <rect
          :width="stageWidth"
          :height="stageHeight"
          rx="8"
          class="stage-rect"
          :style="{ fill: getStageColor(i) }"
        />

        <!-- Step number badge -->
        <circle
          :cx="stageWidth - 8"
          cy="8"
          r="10"
          class="step-badge"
          :style="{ fill: getBadgeColor(i) }"
        />
        <text
          :x="stageWidth - 8"
          y="12"
          text-anchor="middle"
          class="step-number"
        >{{ i + 1 }}</text>

        <!-- Stage name -->
        <text
          :x="stageWidth / 2"
          y="28"
          text-anchor="middle"
          class="stage-name"
        >{{ stage.name }}</text>

        <!-- Stage crate -->
        <text
          :x="stageWidth / 2"
          y="44"
          text-anchor="middle"
          class="stage-crate"
        >{{ stage.crate }}</text>

        <!-- Stage icon (simple text icon) -->
        <text
          :x="stageWidth / 2"
          y="66"
          text-anchor="middle"
          class="stage-icon"
        >{{ stage.icon }}</text>
      </g>

      <!-- Animated dots traveling along connections -->
      <template v-if="isPlaying || dotVisible">
        <circle
          v-for="(dot, i) in animatedDots"
          :key="'dot-' + i"
          :cx="dot.cx"
          :cy="dot.cy"
          r="5"
          fill="url(#dot-gradient)"
          filter="url(#dot-glow)"
          class="flow-dot"
          :style="{ animationDelay: dot.delay + 's' }"
        />
      </template>
    </svg>

    <!-- Description panel -->
    <div class="flow-description">
      <div class="desc-stage-name">
        <span class="desc-number">{{ currentStep + 1 }}.</span>
        {{ stages[currentStep].name }}
      </div>
      <p class="desc-text">{{ stages[currentStep].description }}</p>
      <div class="desc-details">
        <span class="desc-crate">{{ stages[currentStep].crate }}</span>
        <span class="desc-separator">&middot;</span>
        <span class="desc-action">{{ stages[currentStep].action }}</span>
      </div>
    </div>

    <!-- Progress bar -->
    <div class="flow-progress">
      <div
        class="flow-progress-fill"
        :style="{ width: ((currentStep + 1) / stages.length) * 100 + '%' }"
      ></div>
      <div class="flow-progress-steps">
        <div
          v-for="(stage, i) in stages"
          :key="'prog-' + i"
          class="progress-step"
          :class="{
            'progress-active': i === currentStep,
            'progress-done': i < currentStep,
          }"
          @click="jumpToStep(i)"
          :title="stage.name"
        ></div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onUnmounted, watch } from 'vue'

// 8-stage pipeline definition
const stages = [
  {
    name: 'Query',
    crate: 'roko-core',
    icon: '?',
    description: 'A prompt or task enters the system as a Signal. The query is parsed, classified by domain, and tagged with metadata (session, priority, constraints).',
    action: 'Signal creation + classification',
  },
  {
    name: 'Score',
    crate: 'roko-learn',
    icon: '#',
    description: 'Existing knowledge is scored for relevance. The cascade router selects a model tier. Playbook matches and episodic memory are consulted for prior context.',
    action: 'Knowledge scoring + model selection',
  },
  {
    name: 'Route',
    crate: 'roko-gateway',
    icon: '>',
    description: 'The inference gateway routes the request through its nine-stage pipeline: caching, cost checks, backpressure, and provider selection with fallback.',
    action: 'Provider routing + cost accounting',
  },
  {
    name: 'Compose',
    crate: 'roko-compose',
    icon: '+',
    description: 'The 9-layer SystemPromptBuilder assembles the prompt: role template, safety preamble, domain context, knowledge injection, tool descriptions, and task-specific enrichment.',
    action: 'Prompt assembly (9 layers)',
  },
  {
    name: 'Act',
    crate: 'roko-agent',
    icon: '!',
    description: 'The selected provider executes the composed prompt. The tool loop runs: the agent calls tools, receives results, and iterates until completion or budget exhaustion.',
    action: 'LLM dispatch + tool loop',
  },
  {
    name: 'Verify',
    crate: 'roko-gate',
    icon: 'V',
    description: 'The 7-rung gate pipeline validates the output: compile check, test suite, clippy, diff review, oracle evaluation, and safety screening. Adaptive thresholds apply.',
    action: '7-rung gate pipeline',
  },
  {
    name: 'Write',
    crate: 'roko-fs',
    icon: 'W',
    description: 'Verified results are persisted: the signal is appended to the JSONL log, episodes are recorded, learning telemetry is flushed, and knowledge tiers are updated.',
    action: 'Signal + episode persistence',
  },
  {
    name: 'React',
    crate: 'roko-conductor',
    icon: 'R',
    description: 'Reactive watchers fire: the conductor evaluates whether to trigger replanning, update routing weights, run dream consolidation, or publish events to the bus.',
    action: 'Reactive watchers + feedback',
  },
]

// Layout constants
const svgWidth = 1100
const svgHeight = 200
const stageWidth = 110
const stageHeight = 78
const stageGap = 20
const totalStageWidth = stages.length * stageWidth + (stages.length - 1) * stageGap
const startX = (svgWidth - totalStageWidth) / 2
const stageY = (svgHeight - stageHeight) / 2

const stagePositions = computed(() => {
  return stages.map((stage, i) => ({
    ...stage,
    x: startX + i * (stageWidth + stageGap),
    y: stageY,
    centerX: startX + i * (stageWidth + stageGap) + stageWidth / 2,
    centerY: stageY + stageHeight / 2,
  }))
})

const connections = computed(() => {
  const result = []
  for (let i = 0; i < stagePositions.value.length - 1; i++) {
    const from = stagePositions.value[i]
    const to = stagePositions.value[i + 1]
    result.push({
      x1: from.x + stageWidth,
      y1: from.y + stageHeight / 2,
      x2: to.x,
      y2: to.y + stageHeight / 2,
    })
  }
  return result
})

// Animation state
const currentStep = ref(0)
const isPlaying = ref(false)
const dotVisible = ref(false)
let playInterval = null

function togglePlay() {
  if (isPlaying.value) {
    pause()
  } else {
    play()
  }
}

function play() {
  isPlaying.value = true
  dotVisible.value = true
  playInterval = setInterval(() => {
    if (currentStep.value < stages.length - 1) {
      currentStep.value++
    } else {
      currentStep.value = 0
    }
  }, 2000)
}

function pause() {
  isPlaying.value = false
  if (playInterval) {
    clearInterval(playInterval)
    playInterval = null
  }
}

function stepForward() {
  dotVisible.value = true
  if (currentStep.value < stages.length - 1) {
    currentStep.value++
  } else {
    currentStep.value = 0
  }
}

function reset() {
  pause()
  currentStep.value = 0
  dotVisible.value = false
}

function jumpToStep(i) {
  currentStep.value = i
  dotVisible.value = true
}

function isConnectionActive(connIndex) {
  return connIndex === currentStep.value - 1 ||
    (currentStep.value === 0 && connIndex === connections.value.length - 1 && isPlaying.value)
}

// Animated dots along the active connection
const animatedDots = computed(() => {
  if (currentStep.value === 0 && !isPlaying.value) return []
  const connIndex = currentStep.value > 0 ? currentStep.value - 1 : connections.value.length - 1
  const conn = connections.value[connIndex]
  if (!conn) return []

  const dots = []
  for (let d = 0; d < 3; d++) {
    const t = ((Date.now() / 800 + d * 0.33) % 1)
    dots.push({
      cx: conn.x1 + (conn.x2 - conn.x1) * t,
      cy: conn.y1 + (conn.y2 - conn.y1) * t,
      delay: d * 0.15,
    })
  }
  return dots
})

// Re-render dots during animation
let dotFrame = null
const dotTick = ref(0)
function animateDots() {
  dotTick.value++
  dotFrame = requestAnimationFrame(animateDots)
}

watch(isPlaying, (playing) => {
  if (playing) {
    animateDots()
  } else if (dotFrame) {
    cancelAnimationFrame(dotFrame)
    dotFrame = null
  }
})

// Stage colors
function getStageColor(i) {
  const colors = [
    '#3b82f6', // Query - blue
    '#8b5cf6', // Score - violet
    '#6366f1', // Route - indigo
    '#22c55e', // Compose - green
    '#f97316', // Act - orange
    '#ef4444', // Verify - red
    '#14b8a6', // Write - teal
    '#ec4899', // React - pink
  ]
  const base = colors[i] || '#64748b'
  if (i > currentStep.value) return base + '40'
  return base
}

function getBadgeColor(i) {
  if (i === currentStep.value) return '#f59e0b'
  if (i < currentStep.value) return '#22c55e'
  return '#94a3b8'
}

onUnmounted(() => {
  pause()
  if (dotFrame) {
    cancelAnimationFrame(dotFrame)
  }
})
</script>

<style scoped>
.flow-animator {
  width: 100%;
  border: 1px solid var(--vp-c-divider, #e2e8f0);
  border-radius: 8px;
  overflow: hidden;
  background: var(--vp-c-bg, #ffffff);
  margin: 1.5rem 0;
}

.flow-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px;
  border-bottom: 1px solid var(--vp-c-divider, #e2e8f0);
  background: var(--vp-c-bg-soft, #f8fafc);
  flex-wrap: wrap;
  gap: 8px;
}

.flow-title {
  margin: 0;
  font-size: 15px;
  font-weight: 700;
  color: var(--vp-c-text-1, #1e293b);
}

.flow-controls {
  display: flex;
  align-items: center;
  gap: 8px;
}

.flow-controls button {
  padding: 4px 14px;
  border: 1px solid var(--vp-c-divider, #e2e8f0);
  border-radius: 4px;
  background: var(--vp-c-bg, #ffffff);
  color: var(--vp-c-text-1, #1e293b);
  cursor: pointer;
  font-size: 13px;
  font-weight: 500;
  transition: background 0.15s;
}

.flow-controls button:hover:not(:disabled) {
  background: var(--vp-c-bg-elv, #f1f5f9);
}

.flow-controls button:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.flow-step-label {
  font-size: 12px;
  color: var(--vp-c-text-2, #64748b);
  font-weight: 500;
  min-width: 60px;
  text-align: right;
}

.flow-svg {
  width: 100%;
  height: 220px;
  user-select: none;
}

/* Connections */
.flow-connection {
  stroke: #cbd5e1;
  stroke-width: 2;
  stroke-dasharray: 6 3;
  transition: stroke 0.3s, stroke-width 0.3s;
}

.connection-active {
  stroke: #3b82f6;
  stroke-width: 2.5;
  stroke-dasharray: none;
  animation: flow-dash 0.6s linear infinite;
}

@keyframes flow-dash {
  to {
    stroke-dashoffset: -20;
  }
}

/* Stage nodes */
.flow-stage {
  cursor: pointer;
  transition: opacity 0.2s;
}

.stage-rect {
  stroke: transparent;
  stroke-width: 2;
  transition: fill 0.3s, stroke 0.3s;
}

.stage-active .stage-rect {
  stroke: #f59e0b;
  stroke-width: 2.5;
}

.stage-completed .stage-rect {
  opacity: 1;
}

.stage-pending .stage-rect {
  opacity: 0.5;
}

.stage-name {
  fill: #ffffff;
  font-size: 12px;
  font-weight: 700;
  pointer-events: none;
}

.stage-crate {
  fill: rgba(255, 255, 255, 0.7);
  font-size: 9px;
  pointer-events: none;
}

.stage-icon {
  fill: rgba(255, 255, 255, 0.5);
  font-size: 18px;
  font-weight: 700;
  pointer-events: none;
}

.step-badge {
  stroke: rgba(255, 255, 255, 0.3);
  stroke-width: 1;
  transition: fill 0.3s;
}

.step-number {
  fill: #ffffff;
  font-size: 10px;
  font-weight: 700;
  pointer-events: none;
}

/* Animated dots */
.flow-dot {
  opacity: 0.9;
  animation: dot-pulse 0.8s ease-in-out infinite alternate;
}

@keyframes dot-pulse {
  from {
    r: 4;
    opacity: 0.7;
  }
  to {
    r: 6;
    opacity: 1;
  }
}

/* Description panel */
.flow-description {
  padding: 14px 16px;
  border-top: 1px solid var(--vp-c-divider, #e2e8f0);
  min-height: 80px;
}

.desc-stage-name {
  font-size: 15px;
  font-weight: 700;
  color: var(--vp-c-text-1, #1e293b);
  margin-bottom: 4px;
}

.desc-number {
  color: #f59e0b;
  margin-right: 4px;
}

.desc-text {
  margin: 0 0 8px;
  font-size: 13px;
  line-height: 1.6;
  color: var(--vp-c-text-2, #64748b);
}

.desc-details {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
}

.desc-crate {
  display: inline-block;
  padding: 2px 8px;
  background: var(--vp-c-bg-soft, #f1f5f9);
  border-radius: 4px;
  font-family: var(--vp-font-family-mono, monospace);
  font-size: 11px;
  color: var(--vp-c-text-1, #1e293b);
  font-weight: 500;
}

.desc-separator {
  color: var(--vp-c-text-3, #94a3b8);
}

.desc-action {
  color: var(--vp-c-text-2, #64748b);
  font-style: italic;
}

/* Progress bar */
.flow-progress {
  position: relative;
  height: 24px;
  background: var(--vp-c-bg-soft, #f1f5f9);
  border-top: 1px solid var(--vp-c-divider, #e2e8f0);
}

.flow-progress-fill {
  position: absolute;
  top: 0;
  left: 0;
  height: 100%;
  background: linear-gradient(90deg, #3b82f6, #8b5cf6);
  opacity: 0.15;
  transition: width 0.3s ease;
}

.flow-progress-steps {
  position: relative;
  display: flex;
  justify-content: space-around;
  align-items: center;
  height: 100%;
  padding: 0 16px;
}

.progress-step {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: #cbd5e1;
  cursor: pointer;
  transition: background 0.2s, transform 0.2s;
}

.progress-step:hover {
  transform: scale(1.3);
}

.progress-active {
  background: #f59e0b;
  transform: scale(1.3);
}

.progress-done {
  background: #22c55e;
}

/* Dark mode */
.dark .flow-animator {
  border-color: #334155;
}

.dark .flow-header {
  background: #1e293b;
  border-color: #334155;
}

.dark .flow-controls button {
  background: #0f172a;
  border-color: #334155;
  color: #e2e8f0;
}

.dark .flow-controls button:hover:not(:disabled) {
  background: #1e293b;
}

.dark .flow-connection {
  stroke: #475569;
}

.dark .flow-description {
  border-color: #334155;
}

.dark .desc-stage-name {
  color: #f1f5f9;
}

.dark .desc-crate {
  background: #334155;
  color: #e2e8f0;
}

.dark .flow-progress {
  background: #1e293b;
  border-color: #334155;
}

.dark .progress-step {
  background: #475569;
}
</style>
