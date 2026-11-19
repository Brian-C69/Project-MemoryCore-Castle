<script setup>
// The kit's status: a small read-only pill (good, warn, bad or plain) beside one fix button.
defineProps({ tone: { type: String, default: '' } })
</script>

<template>
  <span class="setting-pill" :class="tone"><slot /></span>
</template>

<style scoped>
.setting-pill {
  padding: 2px 9px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--text-dim) 18%, transparent);
  color: var(--text-dim);
  font-size: 11px;
  white-space: nowrap;
}
.good {
  background: color-mix(in srgb, var(--good) 16%, transparent);
  color: var(--good);
}
.warn {
  background: color-mix(in srgb, var(--warn) 16%, transparent);
  color: var(--warn);
}
.bad {
  background: color-mix(in srgb, var(--bad) 16%, transparent);
  color: var(--bad);
}
</style>
