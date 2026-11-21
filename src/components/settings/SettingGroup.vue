<script setup>
// Settings' group level (B): a card with a title, holding 2 to 7 rows. `danger` is the Danger zone
// (always last on its page); `free` pads the card for a whole panel (Database, Backup) instead of rows.
// Its id is a deep-link target: openSettings('chat#pictures').
import { computed, inject } from 'vue'
import HelpTip from '../HelpTip.vue'
import { ROW_STATUS } from './rowstatus.js'

const props = defineProps({
  id: { type: String, default: '' },
  title: { type: String, required: true },
  note: { type: String, default: '' },
  tip: { type: String, default: '' },
  danger: { type: Boolean, default: false },
  free: { type: Boolean, default: false },
})
const status = inject(ROW_STATUS, null)
const lit = computed(() => !!props.id && status?.state.lit === props.id)
</script>

<template>
  <section class="setting-group" :class="{ danger, lit }" :data-setting="id || undefined" :aria-label="title">
    <header class="sg-head">
      <h3>{{ title }}<HelpTip v-if="tip" :text="tip" /></h3>
      <small v-if="note">{{ note }}</small>
      <span class="sg-actions"><slot name="actions" /></span>
    </header>
    <div :class="free ? 'sg-free' : 'sg-rows'"><slot /></div>
  </section>
</template>

<style scoped>
.setting-group {
  border: 1px solid var(--line);
  border-radius: 12px;
  background: color-mix(in srgb, var(--accent) 5%, rgba(0, 0, 0, 0.16));
  transition: box-shadow 0.3s;
}
.setting-group.danger {
  border-color: color-mix(in srgb, var(--bad) 45%, transparent);
}
.setting-group.danger h3 {
  color: var(--bad);
}
.setting-group.lit {
  box-shadow: 0 0 0 2px var(--accent);
}
.sg-head {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 4px 10px;
  padding: 10px 16px;
  border-bottom: 1px solid var(--line);
}
.sg-head h3 {
  display: flex;
  align-items: center;
  margin: 0;
  font-size: 13px;
  font-weight: 600;
  letter-spacing: normal;
}
.sg-head small {
  color: var(--text-dim);
  font-size: 12px;
}
.sg-actions {
  display: flex;
  gap: 8px;
  margin-left: auto;
}
.sg-actions:empty {
  display: none;
}
.sg-free {
  padding: 14px 16px;
}
@media (prefers-reduced-motion: reduce) {
  .setting-group {
    transition: none;
  }
}
</style>
