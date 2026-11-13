<script setup>
// One-click commands for members who do not know MemoryCore's words: each button types its
// command into the session and presses Enter, exactly as if typed. Castle's own buttons follow
// the features the memory folder holds; the member's own buttons come after them.
import { computed, onMounted, ref, watch } from 'vue'
import { getSettings, memoryFeatures } from '../api/castle.js'
import { typeLine } from './typeline.js'

const props = defineProps({
  running: { type: Boolean, default: false },
  // Sends text into the session, as if typed in the Log.
  write: { type: Function, required: true },
  // Changes whenever Settings saves, so the buttons are read again.
  settingsStamp: { type: Number, default: 0 },
})

const features = ref({})
const own = ref([])
const sentLabel = ref('')
let sentTimer

const buttons = computed(() => [
  { label: 'Save diary', command: 'save diary', title: 'Write today into the daily diary' },
  ...(features.value.projects ? [{ label: 'Save project', command: 'save project', title: 'Save the current project to its file' }] : []),
  ...(features.value.saveMemory ? [{ label: 'Save memory', command: 'save memory', title: 'Save what was learned this session' }] : []),
  ...own.value.map((a) => ({ ...a, title: a.command })),
])

async function load() {
  try {
    features.value = await memoryFeatures()
    own.value = (await getSettings()).settings.quickActions ?? []
  } catch {
    // Outside Castle's window: the built-in Save diary only.
  }
}

async function run(button) {
  if (!props.running) return
  try {
    await typeLine(props.write, button.command)
    sentLabel.value = button.label
    clearTimeout(sentTimer)
    sentTimer = setTimeout(() => (sentLabel.value = ''), 2500)
  } catch {
    // Nothing typed; the session may have just ended.
  }
}

onMounted(load)
watch(() => props.settingsStamp, load)
watch(() => props.running, (on) => on && load())
</script>

<template>
  <div class="quick-actions" role="toolbar" aria-label="Quick actions">
    <button
      v-for="button in buttons"
      :key="button.label + button.command"
      :disabled="!running"
      :title="running ? button.title : 'Start the session first'"
      :class="{ sent: sentLabel === button.label }"
      @click="run(button)"
    >{{ sentLabel === button.label ? 'Sent ✓' : button.label }}</button>
  </div>
</template>

<style scoped>
.quick-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  padding: 0 10px 0 0;
}

button {
  padding: 4px 12px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: var(--hover);
  color: var(--text);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}

button.sent {
  border-color: var(--good);
  color: var(--good);
}

button:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

button:hover:not(:disabled) {
  filter: brightness(1.15);
}
</style>
