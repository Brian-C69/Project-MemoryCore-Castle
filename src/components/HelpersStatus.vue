<script setup>
// The helpers' Node in one line, for setup and Settings: whose Node runs pictures, voice, songs,
// video and worlds; a download of Castle's own as it happens; a failure with Try again.
// Castle starts the download by itself at start-up (App.vue); this line only shows it.
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { nodePrepare, nodeStatus, onNodeInstall } from '../api/castle.js'

const state = ref(null)
const busy = ref(false)
let stop

const mb = (bytes) => (bytes / 1048576).toFixed(1)

const line = computed(() => {
  const s = state.value
  if (!s) return { tone: '', text: 'Checking the helpers…' }
  switch (s.state) {
    case 'system':
      return { tone: 'good', text: `Helpers ready - your own Node ${s.version}.` }
    case 'own':
      return { tone: 'good', text: `Helpers ready - Castle's own Node ${s.version}.` }
    case 'fetching':
      return { tone: '', text: `Downloading the helpers once: ${mb(s.done)} / ${mb(s.total)} MB.` }
    case 'failed':
      return { tone: 'warn', text: `The helpers could not be downloaded. ${s.reason}` }
    case 'missing':
      return { tone: 'warn', text: `The helpers are not here yet (about ${Math.round(s.downloadBytes / 1048576)} MB to download).` }
    default:
      return { tone: 'warn', text: 'Pictures, voice and the other helpers need Node.js 20 or later on this computer.' }
  }
})

const canRetry = computed(() => ['failed', 'missing'].includes(state.value?.state))

async function refresh() {
  try {
    state.value = await nodeStatus()
  } catch {
    // outside Castle's window
  }
}

async function retry() {
  busy.value = true
  try {
    state.value = await nodePrepare()
  } catch {
    await refresh()
  } finally {
    busy.value = false
  }
}

onMounted(async () => {
  try {
    stop = await onNodeInstall(({ done, total }) => {
      state.value = { state: 'fetching', done, total }
      // the last bytes: read the outcome once the download has been judged
      if (total && done >= total) setTimeout(refresh, 1500)
    })
  } catch {
    // outside Castle's window
  }
  await refresh()
})
onBeforeUnmount(() => stop?.())
</script>

<template>
  <p class="helpers" :class="line.tone">
    <span>{{ line.text }}</span>
    <button v-if="canRetry" :disabled="busy" @click="retry">{{ busy ? 'Trying…' : 'Try again' }}</button>
  </p>
</template>

<style scoped>
.helpers {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  align-items: center;
  margin: 8px 0;
  font-size: 13px;
  color: var(--muted, var(--text));
}
.good {
  color: var(--good);
}
.warn {
  color: var(--warn);
}
button {
  flex: none;
  padding: 4px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}
button:disabled {
  opacity: 0.5;
  cursor: default;
}
</style>
