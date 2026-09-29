<script setup lang="ts">
/**
 * CrateLink -- links to crate source with description tooltip.
 *
 * Usage in markdown (via global registration):
 *   <CrateLink name="roko-core" desc="Signal + 12 traits, types, config" />
 *   <CrateLink name="roko-agent" desc="12 LLM provider kinds, pools, MCP" path="src/dispatcher/mod.rs" />
 */
import { ref, computed } from 'vue'

const props = withDefaults(
  defineProps<{
    name: string
    desc?: string
    path?: string
    repo?: string
  }>(),
  {
    desc: '',
    path: '',
    repo: 'https://github.com/nunchi/roko',
  }
)

const showTooltip = ref(false)

const href = computed(() => {
  const base = `${props.repo}/tree/main/crates/${props.name}`
  return props.path ? `${base}/${props.path}` : base
})

const tooltipText = computed(() => {
  if (props.desc) return props.desc
  return `crates/${props.name}/`
})
</script>

<template>
  <span
    class="crate-link"
    @mouseenter="showTooltip = true"
    @mouseleave="showTooltip = false"
  >
    <a :href="href" class="crate-link__anchor" target="_blank" rel="noopener">
      {{ name }}
    </a>
    <Transition name="tip">
      <span v-if="showTooltip && tooltipText" class="crate-link__tooltip">
        {{ tooltipText }}
      </span>
    </Transition>
  </span>
</template>

<style scoped>
.tip-enter-active,
.tip-leave-active {
  transition: opacity 0.12s ease;
}
.tip-enter-from,
.tip-leave-to {
  opacity: 0;
}
</style>
