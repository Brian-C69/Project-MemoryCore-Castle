<script setup>
// Sweep's Broom look (design 3): a tall card, a little smiling broom, big soft numbers, a round button.
import { computed } from 'vue'
import { sizeLabel } from './sweep.js'

const props = defineProps({
  total: { type: Number, default: 0 },
  swept: { type: String, default: '' },
  measuring: { type: Boolean, default: false },
  busy: { type: Boolean, default: false },
})
defineEmits(['sweep'])

const parts = computed(() => {
  const [number, unit] = sizeLabel(props.total).split(' ')
  return { number, unit }
})
</script>

<template>
  <div class="broom-look">
    <svg class="broom" :class="{ sweeping: busy }" viewBox="0 0 120 100" aria-hidden="true">
      <g class="sparkles">
        <path d="M22 30 l3 7 7 3 -7 3 -3 7 -3 -7 -7 -3 7 -3z" />
        <path d="M100 38 l2 5 5 2 -5 2 -2 5 -2 -5 -5 -2 5 -2z" />
        <path d="M92 70 l2 4 4 2 -4 2 -2 4 -2 -4 -4 -2 4 -2z" />
      </g>
      <path class="swoosh" d="M14 70 q14 18 40 18" />
      <line class="handle" x1="86" y1="8" x2="66" y2="44" />
      <path class="head" d="M60 40 l18 10 -8 30 q-14 4 -30 -6 z" />
      <path class="face" d="M54 60 q4 4 8 0" />
      <circle class="eye" cx="52" cy="55" r="1.6" />
      <circle class="eye" cx="63" cy="57" r="1.6" />
    </svg>
    <h3>Sweep</h3>
    <p class="size">
      <strong>{{ measuring && !total ? '…' : parts.number }}</strong>
      <span v-if="!(measuring && !total)" class="unit">{{ parts.unit }}</span>
    </p>
    <p class="hint">{{ measuring ? 'measuring' : 'ready to tidy' }}</p>
    <p class="swept">{{ swept }}</p>
    <button class="go" type="button" :disabled="busy" @click="$emit('sweep')">Sweep</button>
  </div>
</template>

<style scoped>
.broom-look {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  width: 190px;
  padding: 14px 16px 18px;
  box-sizing: border-box;
  text-align: center;
}
.broom {
  width: 110px;
  height: 90px;
  fill: none;
  stroke: var(--accent);
  stroke-width: 2.4;
  stroke-linecap: round;
  stroke-linejoin: round;
}
.sparkles path {
  fill: var(--sweep-browser, #38bdf8);
  stroke: none;
}
.swoosh {
  stroke: var(--sweep-browser, #38bdf8);
  opacity: 0.7;
}
.eye {
  fill: var(--accent);
  stroke: none;
}
.sweeping {
  animation: sway 0.9s ease-in-out infinite alternate;
  transform-origin: 70% 20%;
}
@keyframes sway {
  from { transform: rotate(-6deg); }
  to { transform: rotate(6deg); }
}
@media (prefers-reduced-motion: reduce) {
  .sweeping { animation: none; }
}
h3 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
}
.size {
  margin: 0;
}
.size strong {
  font-size: 30px;
  font-weight: 600;
  background: linear-gradient(90deg, var(--accent), var(--sweep-browser, #38bdf8));
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
}
.unit {
  margin-left: 4px;
  font-size: 22px;
  font-weight: 600;
  color: var(--sweep-browser, #38bdf8);
}
.hint,
.swept {
  margin: 0;
  font-size: 12px;
  color: var(--text-dim);
}
.swept {
  padding-top: 6px;
  border-top: 1px solid var(--line);
  width: 100%;
}
.go {
  width: 76px;
  height: 76px;
  margin-top: 8px;
  border: 0;
  border-radius: 50%;
  background: radial-gradient(circle at 35% 30%, var(--sweep-browser, #38bdf8), var(--accent));
  box-shadow: 0 0 24px color-mix(in srgb, var(--accent) 55%, transparent);
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
