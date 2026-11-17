<script setup>
// Sweep's Ring look (design 1): a square card, one ring split by group, the size in its middle.
import { computed } from 'vue'
import { sizeLabel } from './sweep.js'

const props = defineProps({
  total: { type: Number, default: 0 },
  parts: { type: Array, default: () => [] },
  swept: { type: String, default: '' },
  measuring: { type: Boolean, default: false },
  busy: { type: Boolean, default: false },
})
defineEmits(['sweep'])

const RADIUS = 52
const LENGTH = 2 * Math.PI * RADIUS
// each part as a dash along the ring, one after the other
const arcs = computed(() => {
  let at = 0
  return props.parts.map((p) => {
    const arc = { id: p.id, color: p.color, dash: `${p.share * LENGTH} ${LENGTH}`, offset: -at * LENGTH }
    at += p.share
    return arc
  })
})
</script>

<template>
  <div class="ring-look">
    <h3><span class="spark">✦</span>Sweep</h3>
    <div class="ring">
      <svg viewBox="0 0 120 120" aria-hidden="true">
        <circle cx="60" cy="60" :r="RADIUS" class="track" />
        <circle v-for="a in arcs" :key="a.id" cx="60" cy="60" :r="RADIUS" class="arc" :stroke="a.color" :stroke-dasharray="a.dash" :stroke-dashoffset="a.offset" />
      </svg>
      <div class="middle">
        <strong>{{ measuring && !total ? '…' : sizeLabel(total) }}</strong>
        <span>{{ measuring ? 'measuring' : 'can be freed' }}</span>
      </div>
    </div>
    <p class="swept">{{ swept }}</p>
    <button class="go" type="button" :disabled="busy" @click="$emit('sweep')">Sweep</button>
  </div>
</template>

<style scoped>
.ring-look {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  width: 200px;
  padding: 14px 16px 16px;
  box-sizing: border-box;
}
h3 {
  align-self: flex-start;
  margin: 0;
  font-size: 15px;
  font-weight: 500;
}
.spark {
  margin-right: 8px;
  color: var(--accent);
}
.ring {
  position: relative;
  width: 132px;
  height: 132px;
}
svg {
  width: 100%;
  height: 100%;
  transform: rotate(-90deg);
}
circle {
  fill: none;
  stroke-width: 9;
  stroke-linecap: round;
}
.track {
  stroke: var(--line);
  opacity: 0.45;
}
.middle {
  position: absolute;
  inset: 0;
  display: grid;
  place-content: center;
  text-align: center;
}
strong {
  font-size: 22px;
  font-weight: 600;
}
.middle span,
.swept {
  font-size: 12px;
  color: var(--text-dim);
}
.swept {
  margin: 0;
}
.go {
  width: 100%;
  padding: 8px 0;
  border: 0;
  border-radius: 10px;
  background: linear-gradient(90deg, var(--accent), var(--sweep-browser, #38bdf8));
  color: var(--accent-text, #fff);
  font: inherit;
  font-weight: 600;
  cursor: pointer;
}
.go:disabled {
  opacity: 0.6;
  cursor: default;
}
</style>
