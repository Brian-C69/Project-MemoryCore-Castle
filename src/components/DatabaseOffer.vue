<script setup>
// Shown once after an update, to a member whose memory is still in files: would they like it
// in a database? "Set it up for me" installs, starts and creates by itself and then stops -
// the move (which deletes the files once they are safely in the database) is the member's own
// tick in the panel below. "Later" never asks again; Settings > Database stays.
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { dbSetupAll, getSettings, onDbInstall, onDbSetup, saveSettings } from '../api/castle.js'
import { megabytes, progressLine, setupStepLabel } from './database.js'
import DatabasePanel from './DatabasePanel.vue'
import ModalSheet from './ModalSheet.vue'

const props = defineProps({
  companion: { type: String, default: 'your companion' },
  downloadBytes: { type: Number, default: 0 },
})
const emit = defineEmits(['close', 'changed'])

const phase = ref('ask') // ask | working | panel
const steps = ref([]) // [{ step, state }]
const progress = ref('')
const problem = ref('')
let stops = []

// Both buttons mark the offer as seen: it is asked once.
async function seen() {
  try {
    const { settings } = await getSettings()
    if (!settings.dbOfferSeen) await saveSettings({ ...settings, dbOfferSeen: true })
  } catch {
    // outside Castle's window
  }
}

async function later() {
  await seen()
  emit('close')
}

async function setUp() {
  await seen()
  phase.value = 'working'
  problem.value = ''
  steps.value = []
  try {
    await dbSetupAll()
  } catch (reason) {
    problem.value = String(reason)
  }
  // the panel takes over: it shows the counts and the tick, or the step that still needs a hand
  phase.value = 'panel'
  emit('changed')
}

onMounted(async () => {
  try {
    stops.push(
      await onDbSetup(({ step, state }) => {
        const row = steps.value.find((s) => s.step === step)
        if (row) row.state = state
        else steps.value.push({ step, state })
        if (state === 'done' && step === 'install') progress.value = ''
      }),
    )
    stops.push(await onDbInstall(({ done, total }) => (progress.value = progressLine(done, total))))
  } catch {
    // outside Castle's window
  }
})
onBeforeUnmount(() => stops.forEach((stop) => stop()))
</script>

<template>
  <ModalSheet tag="DATABASE" title="Keep your memory in a database?" @close="later">
    <div class="offer">
      <template v-if="phase === 'ask'">
        <p>
          New in this version: {{ companion }}'s memory can live in a small database on this computer instead of in loose files.
        </p>
        <ul>
          <li>Faster to search, and every change is kept.</li>
          <li>Nothing leaves this PC, and your backup carries it.</li>
          <li><strong>Export to markdown</strong> gives every file back whenever you like.</li>
        </ul>
        <p class="hint">
          Castle downloads the database (about {{ megabytes(downloadBytes) }}, no admin needed), starts it and makes {{ companion }}'s own.
          Then it shows you what would move and waits: nothing is moved or deleted until you say so.
        </p>
        <div class="row">
          <button class="primary" @click="setUp">Set it up for me</button>
          <button @click="later">Later</button>
        </div>
        <p class="hint">Later is fine: everything keeps working as it does today, and Settings &gt; Database is always there.</p>
      </template>

      <template v-else-if="phase === 'working'">
        <ol class="steps">
          <li v-for="row in steps" :key="row.step" :class="{ ok: row.state === 'done' }">
            <span class="mark">{{ row.state === 'done' ? '✓' : '…' }}</span> {{ setupStepLabel(row.step) }}
            <span v-if="row.step === 'install' && row.state !== 'done' && progress" class="hint">{{ progress }}</span>
          </li>
        </ol>
        <p class="hint">This takes a minute or two. You can keep using Castle.</p>
      </template>

      <template v-else>
        <p v-if="problem" class="problem">{{ problem }}</p>
        <DatabasePanel :companion="companion" @changed="emit('changed')" />
        <div class="row">
          <button @click="emit('close')">Close</button>
        </div>
      </template>
    </div>
  </ModalSheet>
</template>

<style scoped>
.offer {
  display: flex;
  flex-direction: column;
  gap: 12px;
  max-width: 720px;
  font-size: 15px;
}
p {
  margin: 0;
}
ul {
  display: grid;
  gap: 6px;
  margin: 0;
  padding-left: 20px;
}
.hint {
  font-size: 13px;
  color: var(--text-dim);
}
.problem {
  font-size: 13px;
  color: var(--warn);
}
.steps {
  display: grid;
  gap: 8px;
  margin: 0;
  padding: 0;
  list-style: none;
  color: var(--text-dim);
}
.steps li.ok {
  color: var(--good);
}
.mark {
  display: inline-block;
  width: 20px;
}
.row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
}
button {
  padding: 8px 16px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 14px;
  cursor: pointer;
}
button.primary {
  background: var(--accent);
  border-color: var(--accent);
  color: #07142e;
  font-weight: 600;
}
</style>
