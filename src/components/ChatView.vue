<script setup>
// The chat view arrives in Task 3.3 (lifted from the Keep and taught Codex). Until then the
// panel points the member to the Log, which already carries the whole session.
defineProps({
  running: { type: Boolean, default: false },
  visible: { type: Boolean, default: false },
  write: { type: Function, default: null },
  readScreen: { type: Function, default: null },
  companion: { type: String, default: 'Your companion' },
})
defineEmits(['show-log'])
defineExpose({ addPaths: () => {} })
</script>

<template>
  <div class="chat-soon">
    <p>Chat is on its way.</p>
    <button @click="$emit('show-log')">Open the Log</button>
  </div>
</template>

<style scoped>
.chat-soon {
  margin: auto;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  font-size: 13px;
  color: var(--text-dim);
}
button {
  padding: 6px 14px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: none;
  color: var(--text);
  cursor: pointer;
}
</style>
