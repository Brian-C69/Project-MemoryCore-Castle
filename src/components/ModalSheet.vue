<script setup>
import { onBeforeUnmount, onMounted } from 'vue'

// One window for Castle's readers (a plan, a story part, the board's column): 80% of the
// screen, in the theme's own glass, Castle dimmed and softly blurred behind it - never a black
// page. The header stays; only the body scrolls. It holds Esc while open, so Esc never reaches
defineProps({
  tag: { type: String, default: '' },
  title: { type: String, default: '' },
})
const emit = defineEmits(['close'])

function onKey(event) {
  if (event.key === 'Escape') {
    event.preventDefault()
    event.stopImmediatePropagation()
    emit('close')
  }
}

onMounted(() => window.addEventListener('keydown', onKey, true))
onBeforeUnmount(() => window.removeEventListener('keydown', onKey, true))
</script>

<template>
  <Teleport to="body">
    <div class="sheet-backdrop" role="dialog" aria-modal="true" :aria-label="title || tag" @click.self="emit('close')">
      <article class="sheet-window">
        <header class="sheet-head">
          <span v-if="tag" class="sheet-tag">{{ tag }}</span>
          <span class="sheet-title">{{ title }}</span>
          <slot name="head" />
          <button class="sheet-close" aria-label="Close" @click="emit('close')">&times;</button>
        </header>
        <div class="sheet-body">
          <slot />
        </div>
      </article>
    </div>
  </Teleport>
</template>

<style scoped>
.sheet-backdrop {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(6, 4, 20, 0.42);
  backdrop-filter: blur(3px);
}

.sheet-window {
  display: flex;
  flex-direction: column;
  width: 80vw;
  height: 80vh;
  overflow: hidden;
  border: 1px solid var(--line, rgba(200, 180, 255, 0.38));
  border-radius: 16px;
  background: var(--panel-background, rgba(20, 16, 58, 0.62));
  backdrop-filter: blur(var(--glass-blur, 12px)) saturate(1.1);
  color: var(--text, #f3efff);
  box-shadow: 0 28px 80px rgba(0, 0, 0, 0.5);
  text-align: left;
}

.sheet-head {
  display: flex;
  flex: none;
  align-items: center;
  gap: 14px;
  padding: 14px 20px;
  border-bottom: 1px solid var(--line, rgba(200, 180, 255, 0.25));
  background: rgba(0, 0, 0, 0.12);
}

.sheet-tag {
  flex: none;
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.18em;
  color: var(--accent, #b48cff);
}

.sheet-title {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  font-size: 15px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sheet-close {
  flex: none;
  width: 34px;
  height: 34px;
  border: 0;
  border-radius: 8px;
  background: none;
  color: inherit;
  font-size: 22px;
  cursor: pointer;
}

.sheet-close:hover {
  background: var(--hover, rgba(200, 180, 255, 0.16));
}

.sheet-close:focus-visible {
  outline: 2px solid var(--accent, #b48cff);
}

.sheet-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 20px 30px 30px;
  font-size: 14px;
  line-height: 1.55;
  scrollbar-width: thin;
  scrollbar-color: var(--line, rgba(200, 180, 255, 0.38)) transparent;
}
</style>
