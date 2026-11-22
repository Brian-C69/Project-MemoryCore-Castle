<script setup>
// One line of plain words ("Dentist fri 3pm", "Gym every mon 18:30"). Castle shows what it
// understood before anything is saved, so a wrong guess is visible.
import { onBeforeUnmount, ref, watch } from 'vue'
import { calendarQuickaddPreview } from '../../api/castle.js'

const props = defineProps({ day: { type: String, required: true }, busy: { type: Boolean, default: false } })
const emit = defineEmits(['add', 'more'])
const text = ref('')
const preview = ref(null)
let timer
let token = 0

watch(text, (value) => {
  clearTimeout(timer)
  if (!value.trim()) { preview.value = null; return }
  timer = setTimeout(async () => {
    const mine = ++token
    try {
      const got = await calendarQuickaddPreview(value)
      if (mine === token) preview.value = got
    } catch { if (mine === token) preview.value = null }
  }, 150)
})
onBeforeUnmount(() => clearTimeout(timer))

async function fresh() {
  clearTimeout(timer)
  const mine = ++token
  const got = await calendarQuickaddPreview(text.value)
  if (mine === token) preview.value = got
  return got
}
async function add() {
  if (!text.value.trim() || props.busy) return
  const q = await fresh()
  if (!q.title) { return }
  emit('add', q)
  text.value = ''
  preview.value = null
}
async function more() {
  if (!text.value.trim()) { emit('more', null); return }
  emit('more', await fresh())
  text.value = ''
  preview.value = null
}
</script>

<template>
  <form class="quick" @submit.prevent="add">
    <label class="sr" for="quick-add">Quick add</label>
    <input id="quick-add" v-model="text" type="text" maxlength="300" autocomplete="off" placeholder="Dentist fri 3pm" />
    <p v-if="preview" class="seen" :class="{ warn: !preview.hasDate || !preview.title }" role="status">{{ preview.summary }}<template v-if="!preview.hasDate && preview.title"> - it will go on the day you are looking at</template></p>
    <p v-else class="hint">Type what and when. Try "Gym every mon 18:30" or "Pay rent 1 dec every month".</p>
    <div class="buttons">
      <button type="submit" :disabled="busy || !text.trim() || (preview && !preview.title)">Add</button>
      <button type="button" :disabled="busy" @click="more">More details</button>
    </div>
  </form>
</template>

<style scoped>
.quick { display: grid; gap: 8px; }
.sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); }
input {
  width: 100%; box-sizing: border-box; padding: 10px; color: var(--text);
  background: var(--panel-background); border: 1px solid var(--line); border-radius: 8px; font: inherit;
}
input:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
p { margin: 0; font-size: 12px; line-height: 1.4; }
.hint { color: var(--text-dim); }
.seen { color: var(--text); }
.seen.warn { color: var(--text-dim); }
.buttons { display: flex; gap: 8px; flex-wrap: wrap; }
button {
  padding: 8px 14px; border: 1px solid var(--line); border-radius: 10px;
  background: none; color: var(--text); font: inherit; cursor: pointer;
}
button:hover:not(:disabled) { border-color: color-mix(in srgb, var(--accent) 45%, var(--line)); }
button:disabled { opacity: 0.45; cursor: default; }
button:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
</style>
