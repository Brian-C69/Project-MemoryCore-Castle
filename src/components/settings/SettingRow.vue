<script setup>
// Settings' row level (C-E): the label on the left (5 words or fewer, sentence case, an optional
// HelpTip), one description line under it (what changes, 90 characters at most), the control on the
// right. A row that depends on another setting stays visible and `disabled`, and its description
// says why. "Saved ✓" and a failed save's red line come from the page's row status, by `id`.
// `below` holds what opens under a row (a card editor, a list of notes, a key being pasted).
import { computed, inject } from 'vue'
import HelpTip from '../HelpTip.vue'
import { ROW_STATUS } from './rowstatus.js'

const props = defineProps({
  id: { type: String, default: '' },
  label: { type: String, required: true },
  description: { type: String, default: '' },
  tip: { type: String, default: '' },
  disabled: { type: Boolean, default: false },
})
const status = inject(ROW_STATUS, null)
const saved = computed(() => !!props.id && !!status?.state.saved[props.id])
const error = computed(() => (props.id ? status?.state.errors[props.id] ?? '' : ''))
const lit = computed(() => !!props.id && status?.state.lit === props.id)
</script>

<template>
  <div class="setting-row" :class="{ off: disabled, lit }" :data-setting="id || undefined">
    <div class="sr-text">
      <div class="sr-label">{{ label }}<HelpTip v-if="tip" :text="tip" /></div>
      <div v-if="description || $slots.description" class="sr-desc"><slot name="description">{{ description }}</slot></div>
    </div>
    <div class="sr-control">
      <span v-if="id" class="sr-saved" :class="{ show: saved }" aria-live="polite">{{ saved ? 'Saved ✓' : '' }}</span>
      <slot />
    </div>
    <p v-if="error" class="sr-error" role="alert">{{ error }}</p>
    <div v-if="$slots.below" class="sr-below"><slot name="below" /></div>
  </div>
</template>

<style scoped>
.setting-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 6px 24px;
  align-items: center;
  padding: 11px 16px;
  border-bottom: 1px solid var(--line);
  transition: background-color 0.3s;
}
.setting-row:last-child {
  border-bottom: 0;
}
.setting-row.lit {
  background-color: color-mix(in srgb, var(--accent) 16%, transparent);
}
.sr-label {
  display: flex;
  align-items: center;
  font-size: 13.5px;
}
.sr-desc {
  margin-top: 2px;
  color: var(--text-dim);
  font-size: 12px;
  overflow-wrap: anywhere;
}
.sr-control {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
}
.off .sr-label,
.off .sr-control {
  opacity: 0.45;
}
.sr-saved {
  color: var(--good);
  font-size: 11.5px;
  opacity: 0;
  transition: opacity 0.2s;
}
.sr-saved.show {
  opacity: 1;
}
.sr-error {
  grid-column: 1 / -1;
  margin: 0;
  color: var(--bad);
  font-size: 12px;
}
.sr-below {
  grid-column: 1 / -1;
  min-width: 0;
}
/* a narrow page: the control goes under its label */
@container (max-width: 520px) {
  .setting-row {
    grid-template-columns: minmax(0, 1fr);
  }
  .sr-control {
    justify-content: flex-start;
  }
}
@media (prefers-reduced-motion: reduce) {
  .setting-row,
  .sr-saved {
    transition: none;
  }
}
</style>
