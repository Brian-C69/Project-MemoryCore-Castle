<script setup>
// The kit's typed value: a text box whose Save and Cancel show only while it differs from what is
// saved. Enter saves. The parent saves (`save` with the new words) and hands the saved value back.
import { computed, ref, watch } from 'vue'

const props = defineProps({
  modelValue: { type: [String, null], default: '' },
  label: { type: String, required: true },
  placeholder: { type: String, default: '' },
  maxlength: { type: Number, default: 200 },
  disabled: { type: Boolean, default: false },
})
const emit = defineEmits(['save'])
const draft = ref(props.modelValue ?? '')
watch(() => props.modelValue, (now) => (draft.value = now ?? ''))
const changed = computed(() => draft.value !== (props.modelValue ?? ''))

function save() {
  if (changed.value && !props.disabled) emit('save', draft.value)
}
</script>

<template>
  <span class="setting-text">
    <input v-model="draft" :aria-label="label" :placeholder="placeholder" :maxlength="maxlength" :disabled="disabled" @keydown.enter.prevent="save" />
    <template v-if="changed">
      <button type="button" :disabled="disabled" @click="draft = modelValue ?? ''">Cancel</button>
      <button type="button" class="primary" :disabled="disabled" @click="save">Save</button>
    </template>
  </span>
</template>

<style scoped>
.setting-text {
  display: inline-flex;
  max-width: 100%;
  flex-wrap: wrap;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
}
input {
  width: 220px;
  min-width: 0;
  max-width: 100%;
  padding: 5px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.25);
  color: var(--text);
  font: inherit;
  font-size: 13px;
}
input:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
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
button.primary {
  border-color: var(--accent);
  background: var(--accent);
  color: #07142e;
  font-weight: 600;
}
</style>
