<script setup>
// STUDIO's Workflows rail (design studio-workflows-2): one card per workflow - Castle's own first,
// then the member's and the bought ones. A broken file stays on the rail with its reason.
import { iconOf } from './workflows.js'

defineProps({
  entries: { type: Array, default: () => [] },
  selected: { type: String, default: '' },
})
const emit = defineEmits(['select'])
const nameOf = (e) => e.workflow?.name ?? e.id
const tagOf = (e) => (e.source === 'bought' ? 'Bought' : e.source === 'yours' ? 'Yours' : '')
</script>

<template>
  <nav class="rail" aria-label="Workflows">
    <h4>Workflows</h4>
    <button
      v-for="e in entries"
      :key="e.id"
      class="card"
      :class="{ on: e.id === selected, broken: !!e.error }"
      :title="e.error ?? e.workflow?.description ?? ''"
      :aria-current="e.id === selected ? 'true' : undefined"
      @click="emit('select', e.id)"
    >
      <span class="icon" aria-hidden="true">{{ e.error ? '⚠' : iconOf(e.workflow) }}</span>
      <span class="name">{{ nameOf(e) }}</span>
      <span v-if="tagOf(e)" class="tag">{{ tagOf(e) }}</span>
    </button>
    <slot />
  </nav>
</template>

<style scoped>
.rail {
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-height: 0;
  overflow-y: auto;
  scrollbar-width: var(--scrollbar-width);
  scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track);
}
h4 {
  margin: 0 0 4px;
  font-size: 13px;
  font-weight: 500;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: var(--text-dim);
}
.card {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 10px 12px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: color-mix(in srgb, var(--panel-background) 70%, transparent);
  color: var(--text);
  font: inherit;
  font-size: 14px;
  text-align: left;
  cursor: pointer;
}
.card:hover {
  background: var(--hover);
}
.card.on {
  border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 22%, transparent);
  box-shadow: inset 3px 0 0 var(--accent);
}
.card.broken .name {
  color: var(--warn);
}
.icon {
  display: grid;
  place-items: center;
  width: 30px;
  height: 30px;
  flex-shrink: 0;
  border-radius: 50%;
  background: color-mix(in srgb, var(--accent) 18%, transparent);
  font-size: 15px;
}
.name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.tag {
  padding: 1px 8px;
  border-radius: 999px;
  border: 1px solid var(--line);
  font-size: 10px;
  color: var(--text-dim);
}
</style>
