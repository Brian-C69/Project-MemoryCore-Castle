<script setup>
// A room's picture requests as one line in its top bar: the drawing under way (with Stop), or the
// one that could not be drawn (with Retry and Dismiss). A finished request shows its picture by
// itself (`finished` reloads the room) and is cleared at once, so nothing piles up.
import { computed, ref, onMounted, onBeforeUnmount } from 'vue'
import { imageCancel, imageDismiss, imageJobs, imageRetry } from '../api/castle.js'
import { finishedIds, requestLine } from './imagejobs.js'

const emit = defineEmits(['finished'])
const WORDS = { queued: 'Waiting', working: 'Drawing', switching: 'Waiting', 'prompt-saved': 'Not drawn' }
const jobs = ref([])
const problem = ref('')
const line = computed(() => requestLine(jobs.value))

async function act(work) {
  try { await work(); problem.value = ''; await refresh() } catch (e) { problem.value = String(e?.message ?? e) }
}
const dismiss = (id) => act(() => imageDismiss(id))
const retry = (id) => act(() => imageRetry(id))
const stop = (id) => act(() => imageCancel(id))

let timer
let clearing = false
async function refresh() {
  try {
    jobs.value = await imageJobs()
    const done = finishedIds(jobs.value)
    if (done.length && !clearing) {
      clearing = true
      emit('finished')
      for (const id of done) await imageDismiss(id).catch(() => {})
      clearing = false
      jobs.value = await imageJobs()
    }
  } catch {}
}
onMounted(() => { refresh(); timer = setInterval(refresh, 1500) })
onBeforeUnmount(() => clearInterval(timer))
</script>

<template>
  <span v-if="line || problem" class="request-line" role="status">
    <template v-if="line">
      <strong>{{ WORDS[line.phase] ?? line.phase }}</strong>
      <span v-if="line.label"> {{ line.label }}.</span>
      <span class="message"> {{ line.message }}</span>
      <button v-if="line.phase === 'working'" title="End this drawing; the prompt is kept for a retry" @click="stop(line.id)">Stop</button>
      <template v-if="line.phase === 'prompt-saved'">
        <button @click="retry(line.id)">Retry</button>
        <button title="Take this off" @click="dismiss(line.id)">Dismiss</button>
      </template>
    </template>
    <span v-if="problem" class="problem" role="alert"> {{ problem }}</span>
  </span>
</template>

<style scoped>
.request-line { display: inline-flex; flex-wrap: wrap; align-items: center; gap: 4px 6px; min-width: 0; font-size: 13px; }
.message { color: var(--text-dim); }
button { padding: 2px 8px; border: 1px solid var(--line); border-radius: 6px; background: var(--hover); color: var(--text); font: inherit; cursor: pointer; }
.problem { color: var(--warn, #f0b35a); }
</style>
