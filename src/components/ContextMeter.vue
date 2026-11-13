<script setup>
import { computed } from 'vue'
import { contextTone } from './modes.js'

// How full the session's context window is, as Castle's status line says it: ten segments, neutral,
// then warm near the top. Hidden until the status line has been read once.
const props = defineProps({
  // { percent, tokens } or null
  context: { type: Object, default: null },
})
const lit = computed(() => Math.round((props.context?.percent ?? 0) / 10))
const tone = computed(() => contextTone(props.context?.percent ?? 0))
const title = computed(() => (props.context ? `${props.context.percent}% of the context used - ${props.context.tokens} tokens` : ''))
</script>

<template>
  <span
    v-if="context"
    class="meter"
    :class="tone"
    :title="title"
    role="meter"
    :aria-valuenow="context.percent"
    aria-valuemin="0"
    aria-valuemax="100"
    aria-label="Context used"
  >
    <span class="bar"><span v-for="n in 10" :key="n" class="seg" :class="{ lit: n <= lit }"></span></span>
    <span class="pct">{{ context.percent }}%</span>
  </span>
</template>

<style scoped>
.meter {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  font-variant-numeric: tabular-nums;
}

.bar {
  display: inline-flex;
  gap: 2px;
}

.seg {
  width: 5px;
  height: 10px;
  border-radius: 1px;
  background: color-mix(in srgb, var(--text-dim) 25%, transparent);
}

.neutral .seg.lit {
  background: var(--text-dim);
}

.neutral .pct {
  color: var(--text-dim);
}

.warm .seg.lit {
  background: #e6b07a;
}

.warm .pct {
  color: #e6b07a;
}
</style>
