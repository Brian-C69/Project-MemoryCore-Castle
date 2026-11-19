<script setup>
// Settings' page level (A): one per sidebar item, a title and one line saying what lives here, then
// its groups. A page has no rows outside a group. See docs/settings-standard.md.
defineProps({
  title: { type: String, required: true },
  subtitle: { type: String, default: '' },
})
</script>

<template>
  <section class="settings-page" :aria-label="title">
    <header class="sp-head">
      <h2>{{ title }}</h2>
      <p v-if="subtitle">{{ subtitle }}</p>
    </header>
    <slot />
  </section>
</template>

<style scoped>
.settings-page {
  container-type: inline-size;
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-width: 0;
}
.sp-head h2 {
  margin: 0;
  font-size: 17px;
  font-weight: 600;
  letter-spacing: normal;
}
.sp-head p {
  margin: 2px 0 0;
  color: var(--text-dim);
  font-size: 13px;
}
</style>
