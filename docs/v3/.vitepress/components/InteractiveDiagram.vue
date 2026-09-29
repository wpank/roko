<script setup lang="ts">
/**
 * InteractiveDiagram -- wrapper for zoomable Mermaid diagrams.
 *
 * Provides zoom in/out, reset, and pan controls around a Mermaid
 * diagram slot. The diagram content is passed as the default slot.
 *
 * Usage in markdown:
 *   <InteractiveDiagram title="System Architecture">
 *
 *   ```mermaid
 *   graph TD
 *     A --> B
 *   ```
 *
 *   </InteractiveDiagram>
 */
import { ref, onMounted, onBeforeUnmount } from 'vue'

const props = withDefaults(
  defineProps<{
    title?: string
    minZoom?: number
    maxZoom?: number
  }>(),
  {
    title: 'Diagram',
    minZoom: 0.25,
    maxZoom: 3,
  }
)

const viewport = ref<HTMLElement | null>(null)
const scale = ref(1)
const translateX = ref(0)
const translateY = ref(0)
const isPanning = ref(false)
const startX = ref(0)
const startY = ref(0)

const ZOOM_STEP = 0.15

function zoomIn() {
  scale.value = Math.min(props.maxZoom, scale.value + ZOOM_STEP)
}

function zoomOut() {
  scale.value = Math.max(props.minZoom, scale.value - ZOOM_STEP)
}

function resetView() {
  scale.value = 1
  translateX.value = 0
  translateY.value = 0
}

function onWheel(e: WheelEvent) {
  e.preventDefault()
  if (e.deltaY < 0) {
    zoomIn()
  } else {
    zoomOut()
  }
}

function onPointerDown(e: PointerEvent) {
  isPanning.value = true
  startX.value = e.clientX - translateX.value
  startY.value = e.clientY - translateY.value
  ;(e.target as HTMLElement)?.setPointerCapture?.(e.pointerId)
}

function onPointerMove(e: PointerEvent) {
  if (!isPanning.value) return
  translateX.value = e.clientX - startX.value
  translateY.value = e.clientY - startY.value
}

function onPointerUp() {
  isPanning.value = false
}

function zoomLabel(): string {
  return `${Math.round(scale.value * 100)}%`
}

onMounted(() => {
  viewport.value?.addEventListener('wheel', onWheel, { passive: false })
})

onBeforeUnmount(() => {
  viewport.value?.removeEventListener('wheel', onWheel)
})
</script>

<template>
  <div class="interactive-diagram">
    <div class="interactive-diagram__toolbar">
      <span class="interactive-diagram__title">{{ title }}</span>
      <button @click="zoomOut" title="Zoom out" aria-label="Zoom out">&minus;</button>
      <span class="interactive-diagram__zoom-level">{{ zoomLabel() }}</span>
      <button @click="zoomIn" title="Zoom in" aria-label="Zoom in">&plus;</button>
      <button @click="resetView" title="Reset view" aria-label="Reset view">Reset</button>
    </div>
    <div
      ref="viewport"
      class="interactive-diagram__viewport"
      @pointerdown="onPointerDown"
      @pointermove="onPointerMove"
      @pointerup="onPointerUp"
      @pointercancel="onPointerUp"
    >
      <div
        class="interactive-diagram__content"
        :style="{
          transform: `translate(${translateX}px, ${translateY}px) scale(${scale})`,
          transformOrigin: 'center center',
        }"
      >
        <slot />
      </div>
    </div>
  </div>
</template>

<style scoped>
.interactive-diagram__title {
  font-weight: 600;
  font-size: 0.85rem;
  color: var(--vp-c-text-1);
  margin-right: auto;
}

.interactive-diagram__zoom-level {
  font-size: 0.75rem;
  color: var(--vp-c-text-3);
  min-width: 3em;
  text-align: center;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, monospace;
}

.interactive-diagram__content {
  display: inline-block;
  transition: transform 0.1s ease-out;
  min-width: 100%;
}
</style>
