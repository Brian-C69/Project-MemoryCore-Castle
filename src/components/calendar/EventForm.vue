<script setup>
// The form for one event: add it, change the whole series, skip one day of it, or delete it.
import { computed } from 'vue'
import GlassSelect from '../GlassSelect.vue'
import SettingSwitch from '../settings/SettingSwitch.vue'
import SettingConfirm from '../settings/SettingConfirm.vue'
import { COLOURS, WEEKDAY_CHIPS, jobId, jobOptions, jobValue, linkLeadOptions, remindMinutes, remindOptions, remindValue, repeatText, toEvent } from '../../calendarEvents.js'

const props = defineProps({
  form: { type: Object, required: true },
  occurrence: { type: String, default: '' },
  busy: { type: Boolean, default: false },
  error: { type: String, default: '' },
  // the jobs an event may start; null until they have been read
  jobs: { type: Array, default: null },
})
const emit = defineEmits(['save', 'cancel', 'delete', 'skip', 'approve', 'dismiss'])

const EVERY = [{ value: 'day', label: 'Day' }, { value: 'week', label: 'Week' }, { value: 'month', label: 'Month' }, { value: 'year', label: 'Year' }]
const ENDS = [{ value: 'never', label: 'Never' }, { value: 'until', label: 'On a day' }, { value: 'count', label: 'After a number of times' }]
const saved = computed(() => props.form.id > 0)
const proposal = computed(() => props.form.state === 'proposed')
const remind = computed({ get: () => remindValue(props.form.remindBeforeMin), set: (v) => { props.form.remindBeforeMin = remindMinutes(v) } })
const remindChoices = computed(() => remindOptions(props.form.remindBeforeMin))
const job = computed({ get: () => jobValue(props.form.automationJobId), set: (v) => { props.form.automationJobId = jobId(v); if (props.form.automationJobId == null) props.form.automationLeadMin = 0 } })
const jobChoices = computed(() => jobOptions(props.jobs, props.form.automationJobId))
const leadChoices = computed(() => linkLeadOptions(props.form.automationLeadMin))
const lead = computed({ get: () => String(props.form.automationLeadMin ?? 0), set: (v) => { props.form.automationLeadMin = Number(v) } })
const summary = computed(() => (props.form.repeatOn ? repeatText(toEvent(props.form).repeat) : ''))
function toggleDay(value) {
  const days = props.form.weekdays.includes(value) ? props.form.weekdays.filter((d) => d !== value) : [...props.form.weekdays, value]
  props.form.weekdays = days
}
</script>

<template>
  <form class="event-form" @submit.prevent="emit(proposal ? 'approve' : 'save')">
    <h3>{{ proposal ? 'Suggested event' : saved ? 'Change event' : 'New event' }}</h3>
    <p v-if="proposal" class="hint">Your companion suggested this. It does nothing, and no reminder rings, until you approve it. You can change anything first.</p>
    <label>Title <input v-model="form.title" type="text" maxlength="160" required autofocus /></label>
    <div class="row">
      <label>Day <input v-model="form.date" type="date" required /></label>
      <label class="switch">All day <SettingSwitch v-model="form.allDay" label="All day" /></label>
    </div>
    <div v-if="!form.allDay" class="row">
      <label>Starts <input v-model="form.time" type="time" /></label>
      <label>Ends <input v-model="form.endTime" type="time" /></label>
    </div>
    <div v-if="form.allDay" class="row">
      <label>Last day (if more than one) <input v-model="form.endDate" type="date" :min="form.date" /></label>
    </div>
    <div class="row">
      <label class="switch">Repeats <SettingSwitch v-model="form.repeatOn" label="Repeats" /></label>
    </div>
    <template v-if="form.repeatOn">
      <div class="row">
        <label>Every <input v-model.number="form.interval" type="number" min="1" max="99" class="num" /></label>
        <GlassSelect v-model="form.every" :options="EVERY" aria-label="Repeat unit" />
      </div>
      <div v-if="form.every === 'week'" class="chips" role="group" aria-label="On these days">
        <button v-for="d in WEEKDAY_CHIPS" :key="d.value" type="button" :class="['chip', { on: form.weekdays.includes(d.value) }]" :aria-pressed="form.weekdays.includes(d.value)" @click="toggleDay(d.value)">{{ d.label }}</button>
        <span class="hint">None chosen means the day above.</span>
      </div>
      <div class="row">
        <label>Ends <GlassSelect v-model="form.endsKind" :options="ENDS" aria-label="When the repeat ends" /></label>
        <input v-if="form.endsKind === 'until'" v-model="form.until" type="date" :min="form.date" aria-label="Last day of the repeat" />
        <input v-if="form.endsKind === 'count'" v-model.number="form.count" type="number" min="1" max="1000" class="num" aria-label="Number of times" />
      </div>
      <p class="hint">{{ summary }}</p>
    </template>
    <div class="row">
      <label>Remind me <GlassSelect v-model="remind" :options="remindChoices" aria-label="Remind me" /></label>
      <label>Colour <GlassSelect v-model="form.colour" :options="COLOURS" aria-label="Colour" /></label>
    </div>
    <p v-if="form.remindBeforeMin != null" class="hint">A notice appears on your computer and on HOME while Castle is running. A repeating event reminds before every day of it.</p>
    <div v-if="!proposal && (jobs?.length || form.automationJobId)" class="row">
      <label>Start an automation <GlassSelect v-model="job" :options="jobChoices" aria-label="Start an automation" /></label>
      <label v-if="form.automationJobId">When <GlassSelect v-model="lead" :options="leadChoices" aria-label="When the automation starts" /></label>
    </div>
    <p v-if="!proposal && jobs && !jobs.length && !form.automationJobId" class="hint">An event can start an automation (a briefing before a meeting, say). Make one in AUTOMATIONS with the trigger "Calendar event reaches its time", then choose it here.</p>
    <p v-if="form.automationJobId" class="hint">It starts only if AUTOMATIONS is on and not paused and the job is switched on, and never late: if Castle was closed at that time it is skipped. A repeating event starts it on every day of it.</p>
    <label>Note <textarea v-model="form.note" rows="3" maxlength="4000"></textarea></label>
    <p v-if="error" class="error" role="alert">{{ error }}</p>
    <div class="buttons">
      <button type="submit" :disabled="busy">{{ proposal ? 'Approve' : 'Save' }}</button>
      <button v-if="proposal" type="button" :disabled="busy" @click="emit('dismiss')">Dismiss</button>
      <button type="button" :disabled="busy" @click="emit('cancel')">{{ proposal ? 'Decide later' : 'Cancel' }}</button>
    </div>
    <div v-if="saved && !proposal" class="buttons danger-zone">
      <button v-if="form.repeatOn && occurrence" type="button" :disabled="busy" @click="emit('skip')">Skip this day ({{ occurrence }})</button>
      <SettingConfirm :label="form.repeatOn ? 'Delete the whole repeat' : 'Delete'" :disabled="busy" @confirm="emit('delete')" />
    </div>
    <p v-if="saved && form.repeatOn && !proposal" class="hint">Saving changes every day of the repeat. To move one day, drag it. To leave one day out, use Skip.</p>
  </form>
</template>

<style scoped>
.event-form { display: grid; gap: 12px; }
h3 { margin: 0; font-size: 11px; font-weight: 500; letter-spacing: 0.16em; color: var(--accent); }
label { display: grid; gap: 4px; font-size: 12px; color: var(--text-dim); min-width: 0; }
.row { display: flex; gap: 12px; flex-wrap: wrap; align-items: flex-end; }
.row > label { flex: 1 1 130px; }
label.switch { display: flex; align-items: center; gap: 10px; flex: 0 0 auto; padding-bottom: 8px; }
input, textarea {
  width: 100%; box-sizing: border-box; padding: 9px 10px; color: var(--text);
  background: var(--panel-background); border: 1px solid var(--line); border-radius: 8px; font: inherit;
}
textarea { resize: vertical; }
input.num { width: 6em; }
input:focus-visible, textarea:focus-visible, button:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.chips { display: flex; gap: 6px; flex-wrap: wrap; align-items: center; }
.chip { padding: 4px 10px; border: 1px solid var(--line); border-radius: 20px; background: none; color: var(--text); font: inherit; font-size: 12px; cursor: pointer; opacity: 0.65; }
.chip.on { opacity: 1; border-color: var(--accent); color: var(--accent); }
.hint { margin: 0; font-size: 12px; color: var(--text-dim); line-height: 1.4; }
.error { margin: 0; color: #ef9393; }
.buttons { display: flex; gap: 8px; flex-wrap: wrap; align-items: center; }
.danger-zone { padding-top: 4px; border-top: 1px solid var(--line); }
.buttons button:where(:not(.danger)) {
  padding: 8px 14px; border: 1px solid var(--line); border-radius: 10px;
  background: none; color: var(--text); font: inherit; cursor: pointer;
}
.buttons button:hover:not(:disabled) { border-color: color-mix(in srgb, var(--accent) 45%, var(--line)); }
button:disabled { opacity: 0.45; cursor: default; }
</style>
