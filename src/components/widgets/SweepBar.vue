<script setup>
// Sweep's Bar look (design 2): a wide short card, the size as a sentence, one bar split by group.
import { sizeLabel } from './sweep.js'

defineProps({
  total: { type: Number, default: 0 },
  parts: { type: Array, default: () => [] },
  legend: { type: Array, default: () => [] },
  swept: { type: String, default: '' },
  measuring: { type: Boolean, default: false },
  busy: { type: Boolean, default: false },
})
defineEmits(['sweep'])
</script>

<template>
  <div class="bar-look">
    <div class="top">
      <h3><span class="spark">✦</span>Sweep</h3>
      <button class="go" type="button" :disabled="busy" @click="$emit('sweep')">Sweep</button>
    </div>
    <p class="size">
      <strong>{{ measuring && !total ? 'Measuring…' : sizeLabel(total) }}</strong>
      <span v-if="!(measuring && !total)"> can be freed</span>
    </p>
    <div class="bar" aria-hidden="true">
      <span v-for="p in parts" :key="p.id" :style="{ width: `${p.share * 100}%`, background: p.color }" />
    </div>
    <div class="foot">
      <span v-for="l in legend" :key="l.id" class="key"><i :style="{ background: l.color }" />{{ l.label }}</span>
      <span class="swept">{{ swept }}</span>
    </div>
  </div>
</template>

<style scoped>
.bar-look {
  display: flex;
  flex-direction: column;
  gap: 8px;
  width: 340px;
  padding: 14px 16px;
  box-sizing: border-box;
}
.top {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
h3 {
  margin: 0;
  font-size: 15px;
  font-weight: 500;
}
.spark {
  margin-right: 8px;
  color: var(--accent);
}
.go {
  padding: 6px 16px;
  border: 0;
  border-radius: 999px;
  background: var(--accent);
  color: var(--accent-text, #fff);
  font: inherit;
  font-weight: 600;
  cursor: pointer;
}
.go:disabled {
  opacity: 0.6;
  cursor: default;
}
.size {
  margin: 0;
}
.size strong {
  font-size: 24px;
  font-weight: 600;
}
.size span {
  font-size: 18px;
  color: var(--accent);
}
.bar {
  display: flex;
  height: 8px;
  overflow: hidden;
  border-radius: 999px;
  background: color-mix(in srgb, var(--line) 60%, transparent);
}
.bar span {
  height: 100%;
}
.foot {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 12px;
  align-items: center;
  font-size: 12px;
  color: var(--text-dim);
}
.key i {
  display: inline-block;
  width: 8px;
  height: 8px;
  margin-right: 5px;
  border-radius: 50%;
}
.swept {
  margin-left: auto;
}
</style>
