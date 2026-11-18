<script setup>
// The + beside the six reactions: common emoji to pick from, and a box for any other single emoji
// (typed, pasted, or from Windows' emoji panel with Win + .). Shown only when the room offers
// customReactions. Emits `pick` with one emoji, or `close`.
import { nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { PICKER, isOneEmoji } from '../roomchat.js'

const emit = defineEmits(['pick', 'close'])
const typed = ref('')
const problem = ref('')
const panel = ref(null)
const box = ref(null)

function pick(emoji) {
  emit('pick', emoji)
}

function sendTyped() {
  const emoji = typed.value.trim()
  if (!isOneEmoji(emoji)) {
    problem.value = 'A reaction is one emoji.'
    return
  }
  pick(emoji)
}

function onKey(event) {
  if (event.key === 'Escape') {
    event.stopPropagation()
    emit('close')
  }
}

// a click anywhere outside the panel closes it
function onPointer(event) {
  if (panel.value && !panel.value.contains(event.target)) emit('close')
}

onMounted(() => {
  // after this click has finished, so the + that opened it does not close it again
  setTimeout(() => document.addEventListener('pointerdown', onPointer), 0)
  nextTick(() => panel.value?.querySelector('button')?.focus())
})
onBeforeUnmount(() => document.removeEventListener('pointerdown', onPointer))
</script>

<template>
  <div ref="panel" class="picker" role="dialog" aria-label="Choose a reaction" @keydown="onKey">
    <div class="grid">
      <button v-for="e in PICKER" :key="e" type="button" :aria-label="'React ' + e" @click="pick(e)">{{ e }}</button>
    </div>
    <form class="own" @submit.prevent="sendTyped">
      <input ref="box" v-model="typed" maxlength="16" placeholder="or type or paste one (Win + .)" aria-label="Any one emoji" @input="problem = ''" />
      <button type="submit" :disabled="!typed.trim()">React</button>
    </form>
    <p v-if="problem" class="problem">{{ problem }}</p>
  </div>
</template>

<style scoped>
.picker {
  position: absolute;
  top: 16px;
  z-index: 5;
  width: 268px;
  padding: 8px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: color-mix(in srgb, var(--panel-background) 96%, #050b1f);
  box-shadow: 0 10px 28px rgba(0, 0, 0, 0.45);
  cursor: default;
}
.grid {
  display: grid;
  grid-template-columns: repeat(8, 1fr);
  gap: 2px;
}
.grid button {
  padding: 4px 0;
  border: 0;
  border-radius: 8px;
  background: none;
  font-size: 18px;
  line-height: 1.2;
  cursor: pointer;
}
.grid button:hover,
.grid button:focus-visible {
  background: var(--hover);
}
.own {
  display: flex;
  gap: 6px;
  margin-top: 8px;
}
.own input {
  flex: 1;
  min-width: 0;
  padding: 5px 8px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.25);
  color: var(--text);
  font: inherit;
  font-size: 13px;
}
.own button {
  padding: 5px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 13px;
  cursor: pointer;
}
.own button:disabled {
  opacity: 0.45;
  cursor: default;
}
.problem {
  margin: 6px 0 0;
  color: var(--bad);
  font-size: 12px;
}
</style>
