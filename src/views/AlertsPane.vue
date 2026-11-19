<script setup>
// Settings > Alerts: every thing Castle can tell you about, one group each - a desktop notification,
// a sound, and which sound, as Settings rows (docs/settings-standard.md). The sources come from the
// core (src-tauri/src/alerts.rs), so a source added later shows up here without changing this page.
// CHAT keeps its older settings fields and its two extra choices (which lines, and reactions); every
// other source is saved under `alerts`.
import { computed, inject, onMounted, ref } from 'vue'
import GlassSelect from '../components/GlassSelect.vue'
import SettingChoice from '../components/settings/SettingChoice.vue'
import SettingGroup from '../components/settings/SettingGroup.vue'
import SettingRow from '../components/settings/SettingRow.vue'
import SettingSwitch from '../components/settings/SettingSwitch.vue'
import { ROW_STATUS } from '../components/settings/rowstatus.js'
import { alertSoundPreview, alertSounds, alertSoundsFolder, alertSources } from '../api/castle.js'
import { isMac } from '../platform.js'

// where Castle waits when hidden, and the system's quiet mode: Windows' names, or the Mac's
const hiddenIn = isMac ? 'the menu bar' : 'the tray'
const quietMode = isMac ? 'macOS Focus' : "Windows' Do Not Disturb"
const lookingElsewhere = `Castle tells you when something happens while you are looking elsewhere - in another room, or with Castle in ${hiddenIn}. Each alert stays quiet while you are already looking at the room it is about. ${quietMode} and quiet hours apply to notifications.`

const props = defineProps({
  settings: { type: Object, required: true },
  busy: { type: String, default: '' },
  companionLabel: { type: String, default: 'Your companion' },
  namedTip: { type: String, default: '' },
})
const emit = defineEmits(['save', 'open-room'])

const status = inject(ROW_STATUS, null)
const sources = ref([])
const sounds = ref([{ id: 'glass', label: 'Glass' }])
const soundOptions = computed(() => sounds.value.map((s) => ({ value: s.id, label: s.label })))

async function loadSources() {
  try {
    sources.value = await alertSources()
  } catch {
    // outside Castle: nothing to list
  }
}
async function loadSounds() {
  try {
    sounds.value = await alertSounds()
  } catch {
    // keep the default entry
  }
}
async function openFolder() {
  status?.clear('sounds-folder')
  try {
    await alertSoundsFolder()
  } catch (reason) {
    status?.failed('sounds-folder', reason, 'Not opened')
  }
}

// The rule as it is saved now, for a source: CHAT's older fields, or its entry under `alerts`.
function current(source) {
  if (source.id === 'chat') {
    return { notify: props.settings.alertNotify !== false, sound: !!props.settings.alertSound, soundName: props.settings.alertSoundName || 'glass' }
  }
  const rule = props.settings.alerts?.[source.id] ?? {}
  return { notify: rule.notify ?? source.notify, sound: rule.sound ?? source.sound, soundName: rule.soundName ?? source.soundName }
}

// One change to one source: becomes a settings patch, and the row refreshes from what was saved.
async function change(source, field, value) {
  const now = current(source)
  const row = `${source.id}-${{ notify: 'notify', sound: 'sound', soundName: 'sound-name' }[field]}`
  if (source.id === 'chat') {
    const key = { notify: 'alertNotify', sound: 'alertSound', soundName: 'alertSoundName' }[field]
    emit('save', { [key]: value }, row)
  } else {
    emit('save', { alerts: { ...(props.settings.alerts ?? {}), [source.id]: { notify: now.notify, sound: now.sound, soundName: now.soundName, [field]: value } } }, row)
  }
}
function row(source) {
  const rule = current(source)
  return { ...source, ...rule }
}
const rows = computed(() => sources.value.map(row))
const nothingOn = computed(() => props.settings.alertNotify === false && !props.settings.alertSound)

onMounted(() => {
  loadSources()
  loadSounds()
})
defineExpose({ loadSources })
</script>

<template>
  <SettingGroup v-for="r in rows" :id="`alert-${r.id}`" :key="r.id" :title="r.label" :note="r.about">
    <template #actions>
      <button type="button" class="link" :title="`Open ${r.room.toUpperCase()}`" @click="emit('open-room', r.room)">{{ r.room.toUpperCase() }}</button>
    </template>
    <SettingRow :id="`${r.id}-notify`" label="Desktop notification" :description="`${quietMode} and quiet hours apply.`" :tip="lookingElsewhere">
      <SettingSwitch :model-value="r.notify" label="Desktop notification" :disabled="!!busy" @update:model-value="(on) => change(r, 'notify', on)" />
    </SettingRow>
    <SettingRow :id="`${r.id}-sound`" label="Play a sound">
      <SettingSwitch :model-value="r.sound" label="Play a sound" :disabled="!!busy" @update:model-value="(on) => change(r, 'sound', on)" />
    </SettingRow>
    <SettingRow :id="`${r.id}-sound-name`" label="Sound" :disabled="!r.sound" :description="r.sound ? '' : 'Turn on Play a sound to choose one.'">
      <GlassSelect class="sound-select" :model-value="r.soundName" :options="soundOptions" :disabled="!!busy || !r.sound" :aria-label="`Sound for ${r.label}`" @update:model-value="(v) => change(r, 'soundName', v)" />
      <button type="button" :disabled="!!busy" @click="alertSoundPreview(r.soundName)">Play</button>
    </SettingRow>
    <template v-if="r.id === 'chat'">
      <SettingRow id="chat-when" label="Alert me for" :disabled="nothingOn" :description="nothingOn ? 'Turn on a notification or a sound first.' : ''" :tip="settings.alertWhen === 'named' ? namedTip : ''">
        <SettingChoice :model-value="settings.alertWhen === 'named' ? 'named' : 'all'" :options="[{ value: 'all', label: 'Every message' }, { value: 'named', label: 'When I am named' }]" label="Alert me for" :disabled="!!busy || nothingOn" @update:model-value="(when) => emit('save', { alertWhen: when }, 'chat-when')" />
      </SettingRow>
      <SettingRow id="chat-reactions" label="Reactions to my messages" :disabled="nothingOn" :description="nothingOn ? 'Turn on a notification or a sound first.' : `Also when someone reacts to your message.`">
        <SettingSwitch :model-value="!!settings.alertReactions" label="Reactions to my messages" :disabled="!!busy || nothingOn" @update:model-value="(on) => emit('save', { alertReactions: on }, 'chat-reactions')" />
      </SettingRow>
    </template>
  </SettingGroup>

  <SettingGroup id="own-sounds" title="Your own sounds" tip="Put a .wav file in Castle's sounds folder and it joins the list for every alert above. Only .wav files are offered.">
    <SettingRow id="sounds-folder" label="Sounds folder" description="A .wav file put here joins every list above.">
      <button type="button" @click="openFolder">Open the folder</button>
    </SettingRow>
    <SettingRow id="sounds-refresh" label="Sound list" description="Read the folder again after adding a sound.">
      <button type="button" @click="loadSounds">Refresh sounds</button>
    </SettingRow>
  </SettingGroup>
</template>

<style scoped>
.sound-select {
  min-width: 190px;
}
.link {
  padding: 2px 8px;
  font-size: 11px;
  letter-spacing: 0.08em;
  opacity: 0.8;
}
button {
  padding: 5px 12px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 12.5px;
  cursor: pointer;
}
button:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
</style>
