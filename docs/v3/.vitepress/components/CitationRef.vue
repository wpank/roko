<script setup lang="ts">
/**
 * CitationRef -- hover-preview for academic citations.
 *
 * Usage in markdown (via global registration):
 *   <CitationRef
 *     id="hinton-2006"
 *     authors="Hinton & Salakhutdinov"
 *     year="2006"
 *     title="Reducing the Dimensionality of Data with Neural Networks"
 *     venue="Science 313(5786)"
 *     href="https://arxiv.org/abs/..."
 *   />
 */
import { ref } from 'vue'

const props = defineProps<{
  id: string
  authors: string
  year: string
  title: string
  venue?: string
  href?: string
}>()

const showPreview = ref(false)

function label(): string {
  const surname = props.authors.split(/[,&]/)[0].trim().split(' ').pop()
  return `${surname}, ${props.year}`
}
</script>

<template>
  <span
    class="citation-ref"
    @mouseenter="showPreview = true"
    @mouseleave="showPreview = false"
  >
    <a
      :href="href || `#ref-${id}`"
      class="citation-ref__link"
      :title="`${authors} (${year})`"
    >
      [{{ label() }}]
    </a>
    <Transition name="fade">
      <div v-if="showPreview" class="citation-ref__preview">
        <div class="citation-ref__preview-title">{{ title }}</div>
        <div class="citation-ref__preview-authors">{{ authors }} ({{ year }})</div>
        <div v-if="venue" class="citation-ref__preview-venue">{{ venue }}</div>
      </div>
    </Transition>
  </span>
</template>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.15s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
