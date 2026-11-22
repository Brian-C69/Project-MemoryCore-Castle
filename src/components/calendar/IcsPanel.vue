<script setup>
// Calendar files (.ics): look at what a file holds before anything is added, or choose what to save.
// Everything in a file is drawn as plain text.
import { ref } from 'vue'
import HelpTip from '../HelpTip.vue'
import SettingSwitch from '../settings/SettingSwitch.vue'
import { dayTitle } from '../../calendar.js'
import { repeatText } from '../../calendarEvents.js'

defineProps({
  mode: { type: String, required: true },
  preview: { type: Object, default: null },
  busy: { type: Boolean, default: false },
  error: { type: String, default: '' },
})
const emit = defineEmits(['confirm', 'cancel', 'export'])
const includeDated = ref(false)
function when(item) {
  const [date, time] = (item.startsAt || '').split(' ')
  return `${dayTitle(date)}${item.allDay ? '' : ` ${time}`}`
}
</script>

<template>
  <div class="ics">
    <template v-if="mode === 'export'">
      <h3>Export calendar</h3>
      <p class="hint">Saves your own events as a calendar file (.ics) that other apps can open.<HelpTip block text="It keeps each event's repeat, skipped days and moved days. Suggestions from your companion that you have not approved are left out." /></p>
      <label class="switch">Also include my dated reminders and tasks <SettingSwitch v-model="includeDated" label="Also include my dated reminders and tasks" /></label>
      <p class="hint">The file holds your events' titles and notes, so keep it somewhere private.</p>
      <p v-if="error" class="error" role="alert">{{ error }}</p>
      <div class="buttons">
        <button type="button" :disabled="busy" @click="emit('export', includeDated)">Choose where to save</button>
        <button type="button" :disabled="busy" @click="emit('cancel')">Cancel</button>
      </div>
    </template>
    <template v-else-if="preview">
      <h3>Import {{ preview.fileName }}</h3>
      <p class="sum"><strong>{{ preview.fresh }}</strong> new event{{ preview.fresh === 1 ? '' : 's' }}<template v-if="preview.duplicates">, {{ preview.duplicates }} already in your calendar (skipped)</template>.</p>
      <p v-if="preview.ownDated" class="hint">{{ preview.ownDated }} reminder{{ preview.ownDated === 1 ? '' : 's' }} or task{{ preview.ownDated === 1 ? '' : 's' }} that came from Castle itself {{ preview.ownDated === 1 ? 'was' : 'were' }} left out: they already live in Castle.</p>
      <ul v-if="preview.warnings.length" class="warn" aria-label="Things to know">
        <li v-for="(w, i) in preview.warnings" :key="i">{{ w }}</li>
      </ul>
      <p class="hint">A file cannot set a reminder, start an automation or instruct your companion. Nothing is added until you press Import.</p>
      <ul class="list">
        <li v-for="(item, i) in preview.shown" :key="i" :class="{ dup: item.duplicate }">
          <span class="what"><span class="title">{{ item.title }}</span><span class="sub">{{ when(item) }}<template v-if="item.repeat"> · {{ repeatText(item.repeat).toLowerCase() }}</template><template v-if="item.duplicate"> · already here</template></span></span>
        </li>
      </ul>
      <p v-if="preview.more" class="hint">...and {{ preview.more }} more.</p>
      <p v-if="error" class="error" role="alert">{{ error }}</p>
      <div class="buttons">
        <button type="button" :disabled="busy || !preview.fresh" @click="emit('confirm')">Import {{ preview.fresh }} event{{ preview.fresh === 1 ? '' : 's' }}</button>
        <button type="button" :disabled="busy" @click="emit('cancel')">Cancel</button>
      </div>
    </template>
  </div>
</template>

<style scoped>
.ics { display: grid; gap: 12px; }
h3 { margin: 0; font-size: 11px; font-weight: 500; letter-spacing: 0.16em; color: var(--accent); overflow-wrap: anywhere; }
.hint { margin: 0; font-size: 12px; color: var(--text-dim); line-height: 1.4; }
.sum { margin: 0; }
.error { margin: 0; color: #ef9393; }
label.switch { display: flex; align-items: center; gap: 10px; font-size: 12px; color: var(--text-dim); }
ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 6px; }
.warn { padding: 8px 10px; border: 1px dashed var(--line); border-radius: 10px; font-size: 12px; line-height: 1.4; color: var(--text-dim); }
.list { max-height: 260px; overflow-y: auto; scrollbar-width: var(--scrollbar-width); scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track); }
.list li { padding: 6px 8px; border: 1px solid var(--line); border-radius: 8px; }
.list li.dup { opacity: 0.55; }
.what { display: grid; gap: 2px; min-width: 0; }
.title { overflow-wrap: anywhere; }
.sub { font-size: 11.5px; color: var(--text-dim); }
.buttons { display: flex; gap: 8px; flex-wrap: wrap; }
button { padding: 8px 14px; border: 1px solid var(--line); border-radius: 10px; background: none; color: var(--text); font: inherit; cursor: pointer; }
button:hover:not(:disabled) { border-color: color-mix(in srgb, var(--accent) 45%, var(--line)); }
button:disabled { opacity: 0.45; cursor: default; }
button:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
</style>
