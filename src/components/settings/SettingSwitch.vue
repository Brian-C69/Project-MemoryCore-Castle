<script setup>
// The kit's toggle: an on/off switch that saves at once. It shows the value it is given, so a save
// that fails leaves it where it was.
defineProps({
  modelValue: { type: Boolean, default: false },
  label: { type: String, required: true },
  disabled: { type: Boolean, default: false },
})
const emit = defineEmits(['update:modelValue'])
</script>

<template>
  <button type="button" class="setting-switch" role="switch" :aria-checked="modelValue" :aria-label="label" :disabled="disabled" @click="emit('update:modelValue', !modelValue)"></button>
</template>

<style scoped>
.setting-switch {
  position: relative;
  flex: none;
  width: 34px;
  height: 20px;
  padding: 0;
  border: 0;
  border-radius: 999px;
  background: color-mix(in srgb, var(--text-dim) 35%, transparent);
  cursor: pointer;
  transition: background-color 0.15s;
}
.setting-switch::after {
  content: '';
  position: absolute;
  top: 3px;
  left: 3px;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: #fff;
  transition: left 0.15s;
}
.setting-switch[aria-checked='true'] {
  background: var(--accent);
}
.setting-switch[aria-checked='true']::after {
  left: 17px;
}
.setting-switch:disabled {
  cursor: not-allowed;
}
.setting-switch:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
@media (prefers-reduced-motion: reduce) {
  .setting-switch,
  .setting-switch::after {
    transition: none;
  }
}
</style>
