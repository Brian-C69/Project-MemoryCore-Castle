<script setup>
// "What's new in Castle X": shown once after an update, and from Settings > About any time.
import ModalSheet from './ModalSheet.vue'
import MarkdownText from './MarkdownText.vue'
import { openUrl } from '../api/castle.js'

defineProps({
  version: { type: String, default: '' },
  sections: { type: Array, default: () => [] }, // [{ version, date, body }], newest first
})
const emit = defineEmits(['close'])
const RELEASES = 'https://github.com/Kiyoraka/Project-MemoryCore-Castle/releases'
</script>

<template>
  <ModalSheet tag="WHAT'S NEW" :title="`MemoryCore Castle ${version}`" @close="emit('close')">
    <div class="whats-new">
      <section v-for="s in sections" :key="s.version">
        <h3 v-if="sections.length > 1 || s.version !== version">{{ s.version }}<span v-if="s.date" class="date"> · {{ s.date }}</span></h3>
        <MarkdownText :text="s.body" size="reader" />
      </section>
      <p v-if="!sections.length" class="hint">No notes for this version.</p>
      <footer>
        <button class="link" @click="openUrl(RELEASES).catch(() => {})">All releases</button>
        <button class="close" @click="emit('close')">Got it</button>
      </footer>
    </div>
  </ModalSheet>
</template>

<style scoped>
.whats-new {
  max-width: 760px;
  margin: 0 auto;
}
h3 {
  margin: 18px 0 6px;
  font-weight: 500;
  letter-spacing: 0.04em;
}
.date,
.hint {
  color: var(--text-dim);
}
footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 24px;
}
button {
  padding: 6px 16px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: transparent;
  color: var(--text);
  font: inherit;
  cursor: pointer;
}
button.close {
  background: var(--accent);
  color: #07142e;
}
button:hover {
  background: var(--hover);
}
button.close:hover {
  filter: brightness(1.1);
}
</style>
