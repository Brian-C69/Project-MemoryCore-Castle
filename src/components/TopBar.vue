<script setup>
import TopMusic from './TopMusic.vue'

defineProps({
  tabs: { type: Array, required: true },
  active: { type: String, required: true },
  // Tabs with something new waiting (the story room speaking, a new chat line): their button changes colour.
  marks: { type: Array, default: () => [] },
  // what is new, per tab, for its tooltip ("2 new · Rin online")
  markTitles: { type: Object, default: () => ({}) },
})
defineEmits(['select', 'settings', 'hide'])
</script>

<template>
  <header class="top-bar">
    <span class="title"><span class="gem">◆</span> MEMORYCORE</span>
    <nav class="tabs">
      <button
        v-for="tab in tabs"
        :key="tab.id"
        class="tab"
        :class="{ active: tab.id === active, fresh: marks.includes(tab.id) }"
        :title="marks.includes(tab.id) ? markTitles[tab.id] || 'Something new is waiting' : undefined"
        @click="$emit('select', tab.id)"
      >
        {{ tab.label }}
      </button>
    </nav>
    <div class="actions">
      <TopMusic />
      <button class="action" title="Settings" aria-label="Settings" @click="$emit('settings')">&#9881;</button>
      <button class="action" title="Hide to tray" aria-label="Hide to tray" @click="$emit('hide')">&#8211;</button>
    </div>
  </header>
</template>

<style scoped>
.top-bar {
  display: flex;
  align-items: center;
  gap: 40px;
  height: var(--bar-height);
  padding: 0 20px;
  background: var(--bar-background);
  border-bottom: 1px solid var(--line);
  color: var(--text);
}

/* Something new is waiting there: gold-violet, a small dot. */
.tab.fresh {
  position: relative;
  color: #fff8e8;
  background: linear-gradient(135deg, rgba(138, 92, 240, 0.85), rgba(199, 154, 74, 0.85));
  border-radius: 8px;
  box-shadow: 0 0 14px rgba(230, 194, 122, 0.5);
}

.tab.fresh::after {
  content: '';
  position: absolute;
  top: 4px;
  right: 4px;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: #ffe39a;
  box-shadow: 0 0 6px #ffe39a;
}

.title {
  font-size: 13px;
  letter-spacing: 0.2em;
  white-space: nowrap;
}

.gem {
  margin-right: 6px;
  color: var(--accent);
}

.tabs {
  display: flex;
  flex: 1;
  gap: 8px;
}

button {
  border: 0;
  background: none;
  color: inherit;
  font: inherit;
  cursor: pointer;
}

.tab {
  padding: 8px 14px;
  font-size: 13px;
  letter-spacing: 0.14em;
  color: var(--text-dim);
  border-bottom: 2px solid transparent;
}

.tab:hover {
  color: var(--text);
}

.tab.active {
  color: var(--text);
  border-bottom-color: var(--accent);
}

.actions {
  display: flex;
  gap: 4px;
}

.action {
  width: 32px;
  height: 32px;
  font-size: 16px;
  border-radius: 6px;
  color: var(--text-dim);
}

.action:hover {
  color: var(--text);
  background: var(--hover);
}

button:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
</style>
