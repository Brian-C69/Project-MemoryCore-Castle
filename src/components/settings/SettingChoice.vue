<script setup>
// The kit's choice: 2 to 4 options side by side, one pressed, saved at once. Five or more options
// take a GlassSelect instead. Options are [{ value, label, title }].
defineProps({
  modelValue: { type: [String, Number, Boolean, null], default: null },
  options: { type: Array, required: true },
  label: { type: String, required: true },
  disabled: { type: Boolean, default: false },
})
const emit = defineEmits(['update:modelValue'])
</script>

<template>
  <div class="setting-choice" role="group" :aria-label="label">
    <button v-for="o in options" :key="String(o.value)" type="button" :aria-pressed="modelValue === o.value" :title="o.title || undefined" :disabled="disabled" @click="modelValue !== o.value && emit('update:modelValue', o.value)">{{ o.label }}</button>
  </div>
</template>

<style scoped>
.setting-choice {
  display: inline-flex;
  flex-wrap: wrap;
  gap: 2px;
  padding: 2px;
  border: 1px solid var(--line);
  border-radius: 999px;
}
.setting-choice button {
  padding: 4px 11px;
  border: 0;
  border-radius: 999px;
  background: transparent;
  color: var(--text-dim);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}
.setting-choice button[aria-pressed='true'] {
  background: var(--accent);
  color: #07142e;
  font-weight: 600;
}
.setting-choice button:disabled {
  cursor: not-allowed;
  opacity: 0.6;
}
.setting-choice button:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
}
</style>
