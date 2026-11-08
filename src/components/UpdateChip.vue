<script setup>
// "Update ready" on the top bar (official builds only). Update now saves the diary first: the
// companion is asked to save it, and Castle waits until the diary changes (three minutes at most,
// or Skip). Then the update downloads, its signature is checked, it installs and Castle restarts.
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { diaryStamp, terminalState, terminalWrite, updateCheck, updateInstall } from '../api/castle.js'

const emit = defineEmits(['status'])
const status = ref(null)
const step = ref('')
let skip = false
let timer

const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

async function check() {
  try {
    status.value = await updateCheck()
    emit('status', status.value)
  } catch {
    // no update line
  }
}

async function saveDiaryFirst() {
  const state = await terminalState().catch(() => null)
  if (!state?.running) return
  const before = await diaryStamp()
  step.value = 'Saving the diary first…'
  await terminalWrite('save diary')
  await terminalWrite('\r')
  for (let waited = 0; waited < 180 && !skip; waited += 3) {
    await sleep(3000)
    if ((await diaryStamp()) > before) {
      // give the companion a moment to finish writing
      await sleep(5000)
      return
    }
  }
}

async function updateNow() {
  skip = false
  try {
    await saveDiaryFirst()
    step.value = 'Downloading the update…'
    await updateInstall()
  } catch (reason) {
    step.value = String(reason)
  }
}

onMounted(() => {
  check()
  timer = setInterval(check, 6 * 60 * 60 * 1000)
})
onBeforeUnmount(() => clearInterval(timer))
</script>

<template>
  <div v-if="status?.available" class="update">
    <template v-if="!step">
      <span>Update ready: {{ status.available }}</span>
      <button @click="updateNow">Update now</button>
    </template>
    <template v-else>
      <span>{{ step }}</span>
      <button v-if="step.startsWith('Saving')" @click="skip = true">Skip</button>
    </template>
  </div>
</template>

<style scoped>
.update {
  position: fixed;
  top: 8px;
  right: 110px;
  z-index: 40;
  display: flex;
  gap: 8px;
  align-items: center;
  padding: 4px 10px;
  border: 1px solid var(--good);
  border-radius: 999px;
  background: var(--panel-background);
  font-size: 12px;
}
button {
  padding: 2px 10px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: var(--good);
  color: #07142e;
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}
</style>
