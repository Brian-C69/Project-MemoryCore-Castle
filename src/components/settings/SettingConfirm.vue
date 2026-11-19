<script setup>
// The kit's danger button: it asks in place ("Sure?" with Keep beside it) instead of a confirm()
// box, and only the second press does it.
import { ref } from 'vue'

defineProps({
  label: { type: String, required: true },
  sure: { type: String, default: '' },
  disabled: { type: Boolean, default: false },
})
const emit = defineEmits(['confirm'])
const asking = ref(false)
</script>

<template>
  <span class="setting-confirm">
    <button v-if="!asking" type="button" class="danger" :disabled="disabled" @click="asking = true">{{ label }}</button>
    <template v-else>
      <span class="sure">Sure?</span>
      <button type="button" @click="asking = false">Keep</button>
      <button type="button" class="danger" :disabled="disabled" @click="asking = false; emit('confirm')">{{ sure || label }}</button>
    </template>
  </span>
</template>

<style scoped>
.setting-confirm {
  display: inline-flex;
  align-items: center;
  gap: 8px;
}
.sure {
  color: var(--bad);
  font-size: 12px;
}
button {
  padding: 5px 12px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}
button.danger {
  border-color: var(--bad);
  color: var(--bad);
}
button:disabled {
  cursor: not-allowed;
  opacity: 0.45;
}
</style>
