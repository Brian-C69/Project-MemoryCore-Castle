<script setup>
// HOME's AUTOMATIONS card: what your automations last reported (a result, a failure, a phone
// delivery that did not arrive) and whether they are paused. It is a card like the others: drag it
// anywhere on HOME, size it in WIDGETS. It only reads what the room already loaded.
import { computed } from 'vue'
import RunnerHealth from '../automations/RunnerHealth.vue'
import { noticeLine } from '../automations/automations.js'

// Widgets are handed the whole data bundle; only declared props are used.
defineOptions({ inheritAttrs: false })
const props = defineProps({ data: { type: Object, default: null } })
defineEmits(['open'])

const paused = computed(() => Boolean(props.data?.paused))
const rows = computed(() => (props.data?.notices || []).slice(0, 5).map(noticeLine))
</script>

<template>
  <div class="widget automations-card">
    <div class="head">
      <span class="title">AUTOMATIONS</span>
      <span v-if="paused" class="paused">Paused</span>
    </div>
    <RunnerHealth :health="data?.health" compact />
    <p v-if="!rows.length" class="none">{{ paused ? 'All automations are paused.' : 'Nothing to report. Results and failures show here.' }}</p>
    <ul v-else class="notes">
      <li v-for="(n, i) in rows" :key="i" :title="n.text">
        <div class="top">
          <strong>{{ n.name }}</strong>
          <span class="state" :class="n.tone">{{ n.state }}</span>
        </div>
        <p class="line">{{ n.text }}</p>
      </li>
    </ul>
    <button type="button" class="open" @click="$emit('open')">Open AUTOMATIONS</button>
  </div>
</template>

<style scoped>
.automations-card {
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-width: 250px;
  max-width: 300px;
  padding: 14px 16px;
}
.head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 10px;
}
.title {
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0.14em;
  color: var(--text-dim);
}
.paused {
  font-size: 11px;
  color: #e0b25c;
}
.none {
  margin: 0;
  font-size: 13px;
  color: var(--text-dim);
}
.notes {
  display: flex;
  flex-direction: column;
  gap: 10px;
  max-height: 260px;
  margin: 0;
  padding: 0 4px 0 0;
  overflow-y: auto;
  list-style: none;
  scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track);
  scrollbar-width: var(--scrollbar-width);
}
li {
  display: flex;
  flex-direction: column;
  gap: 3px;
}
.top {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 8px;
  min-width: 0;
}
.top strong {
  min-width: 0;
  overflow: hidden;
  font-size: 13px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.state {
  flex: none;
  font-size: 11px;
  color: var(--text-dim);
}
.state.bad {
  color: #ef9393;
}
.line {
  display: -webkit-box;
  margin: 0;
  overflow: hidden;
  font-size: 12px;
  line-height: 1.4;
  color: var(--text-dim);
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 3;
}
.open {
  align-self: flex-start;
  padding: 6px 12px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}
.open:hover {
  border-color: color-mix(in srgb, var(--accent) 45%, var(--line));
}
.open:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
</style>
