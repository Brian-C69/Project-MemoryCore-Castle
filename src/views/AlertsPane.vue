<script setup>
// Settings > Alerts: every thing Castle can tell you about, one card each - a desktop notification,
// a sound, and which sound. The sources come from the core (src-tauri/src/alerts.rs), so a source
// added later shows up here without changing this page. CHAT keeps its older settings fields and
// its two extra choices (which lines, and reactions); every other source is saved under `alerts`.
import { computed, onMounted, ref } from 'vue'
import GlassSelect from '../components/GlassSelect.vue'
import HelpTip from '../components/HelpTip.vue'
import { alertSoundPreview, alertSounds, alertSoundsFolder, alertSources } from '../api/castle.js'
import { isMac } from '../platform.js'

// where Castle waits when hidden, and the system's quiet mode: Windows' names, or the Mac's
const hiddenIn = isMac ? 'the menu bar' : 'the tray'
const quietMode = isMac ? 'macOS Focus' : "Windows' Do Not Disturb"

const props = defineProps({
  settings: { type: Object, required: true },
  busy: { type: String, default: '' },
  companionLabel: { type: String, default: 'Your companion' },
  namedTip: { type: String, default: '' },
})
const emit = defineEmits(['save', 'open-room'])

const sources = ref([])
const sounds = ref([{ id: 'glass', label: 'Glass' }])
const problem = ref('')
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
  problem.value = ''
  try {
    await alertSoundsFolder()
  } catch (reason) {
    problem.value = String(reason?.message ?? reason)
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
  if (source.id === 'chat') {
    const key = { notify: 'alertNotify', sound: 'alertSound', soundName: 'alertSoundName' }[field]
    emit('save', { [key]: value })
  } else {
    emit('save', { alerts: { ...(props.settings.alerts ?? {}), [source.id]: { notify: now.notify, sound: now.sound, soundName: now.soundName, [field]: value } } })
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
  <section class="alerts-pane">
    <p class="intro">
      Castle can tell you when something happens while you are looking elsewhere - in another room, or with Castle
      in {{ hiddenIn }}. Each kind of alert has its own notification, sound and sound choice, and stays quiet while you are
      already looking at the room it is about. {{ quietMode }} and quiet hours apply to notifications.
    </p>

    <article v-for="r in rows" :key="r.id" class="alert-card">
      <header>
        <h3>{{ r.label }}</h3>
        <button type="button" class="link" :title="`Open ${r.room.toUpperCase()}`" @click="emit('open-room', r.room)">{{ r.room.toUpperCase() }}</button>
      </header>
      <p class="about">{{ r.about }}</p>

      <div class="controls">
        <label class="toggle"><input type="checkbox" :checked="r.notify" :disabled="!!busy" @change="change(r, 'notify', $event.target.checked)" /> Desktop notification</label>
        <label class="toggle"><input type="checkbox" :checked="r.sound" :disabled="!!busy" @change="change(r, 'sound', $event.target.checked)" /> Play a sound</label>
        <div class="sound-pick">
          <GlassSelect class="sound-select" :model-value="r.soundName" :options="soundOptions" :disabled="!!busy || !r.sound" :aria-label="`Sound for ${r.label}`" @update:model-value="(v) => change(r, 'soundName', v)" />
          <button type="button" :disabled="!!busy" @click="alertSoundPreview(r.soundName)">Play</button>
        </div>
      </div>

      <template v-if="r.id === 'chat'">
        <fieldset class="alert-when" :disabled="!!busy || nothingOn">
          <legend>Alert me for</legend>
          <label class="toggle"><input type="radio" name="alert-when" value="all" :checked="settings.alertWhen !== 'named'" @change="emit('save', { alertWhen: 'all' })" /> Every new message</label>
          <div class="setting-row">
            <label class="toggle"><input type="radio" name="alert-when" value="named" :checked="settings.alertWhen === 'named'" @change="emit('save', { alertWhen: 'named' })" /> Only when someone names me or {{ companionLabel }}</label>
            <HelpTip :text="namedTip" />
          </div>
        </fieldset>
        <label class="toggle"><input type="checkbox" :checked="!!settings.alertReactions" :disabled="!!busy || nothingOn" @change="emit('save', { alertReactions: $event.target.checked })" /> Also when someone reacts to my message</label>
      </template>
    </article>

    <article class="alert-card">
      <header><h3>Your own sounds</h3></header>
      <p class="about">Put a <code>.wav</code> file in Castle's sounds folder and it joins the list for every alert above. Only .wav files are offered.</p>
      <div class="controls">
        <button type="button" @click="openFolder">Open the sounds folder</button>
        <button type="button" @click="loadSounds">Refresh sounds</button>
      </div>
      <p v-if="problem" class="hint">{{ problem }}</p>
    </article>
  </section>
</template>

<style scoped>
.alerts-pane {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.intro {
  margin: 0;
  color: var(--text-dim);
  font-size: 13px;
  line-height: 1.5;
}
.alert-card {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px 14px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: color-mix(in srgb, var(--accent) 6%, transparent);
}
.alert-card header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 10px;
}
.alert-card h3 {
  margin: 0;
  font-size: 14px;
}
.about {
  margin: 0;
  color: var(--text-dim);
  font-size: 12.5px;
  line-height: 1.45;
}
.controls {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px 18px;
}
.sound-pick {
  display: flex;
  align-items: center;
  gap: 8px;
}
.sound-select {
  min-width: 190px;
}
.link {
  padding: 2px 8px;
  font-size: 11px;
  letter-spacing: 0.08em;
  opacity: 0.8;
}
.alert-when {
  display: flex;
  flex-direction: column;
  gap: 8px;
  border: 1px solid var(--line);
  border-radius: 10px;
  padding: 8px 12px;
}
.alert-when legend {
  padding: 0 6px;
  font-size: 12px;
  color: var(--text-dim);
}
.setting-row {
  display: flex;
  align-items: center;
  gap: 2px;
}
.hint {
  margin: 0;
  font-size: 12px;
  color: var(--text-dim);
}
</style>
