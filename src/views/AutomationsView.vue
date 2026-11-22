<script setup>
import { onMounted, onBeforeUnmount, ref, computed, watch } from 'vue'
import { automationSave, automationRun, automationDescribe, automationMessageMatch, automationHistory, automationArchive, automationPause, automationHookReset, chooseFolder } from '../api/castle.js'
import { automations, prepareAutomations } from '../composables/useAutomations.js'
import { newJob, triggerSummary, pill, nextRun, commandFields, actionSummary, CASTLE_ACTIONS, DESTINATIONS, castleAction, toggleDestination, runStatus, triggerDefaults, listeningText, EVENTS, requestBody, savedTrigger } from '../components/automations/automations.js'
import RunnerHealth from '../components/automations/RunnerHealth.vue'
import ModalSheet from '../components/ModalSheet.vue'
import GlassSelect from '../components/GlassSelect.vue'
import HelpTip from '../components/HelpTip.vue'
import SettingSwitch from '../components/settings/SettingSwitch.vue'
import '../components/automations/roomcard.css'
const emit = defineEmits(['settings'])
const triggerKind = ref('schedule'), sampleText = ref('Sample invoice message'), matchResult = ref(null)
const describeOpen = ref(false), description = ref(''), generated = ref(false), bodyText = ref('null')
const followOptions = computed(() => [{value:'',label:'No follow-up'}, ...(automations.value?.jobs || []).filter(j => j.state === 'active' && j.id !== editor.value?.id).map(j => ({value:j.name,label:j.name + (j.enabled ? '' : ' (off)')}))])
const actionKind = ref('companion'), argsText = ref('[]'), envText = ref('{}')
const programOptions = computed(() => automations.value?.programs || [])
const archived = ref(false), hookUrl = ref(''), linkCopied = ref(false)
const visibleJobs = computed(() => (automations.value?.jobs || []).filter(j => archived.value ? j.state === 'archived' : j.state !== 'archived'))
const loadError = ref('')
const error = ref(''), busy = ref(false), editor = ref(null), history = ref(null), runs = ref([]), scheduleType = ref('cron'), scheduleValue = ref(''), testRun = ref(null)
watch(() => [editor.value?.trigger?.text, editor.value?.trigger?.match, editor.value?.trigger?.case, editor.value?.trigger?.platform, sampleText.value], () => { matchResult.value = null })
let timer
async function load() { try { await prepareAutomations(); if (history.value) runs.value = await automationHistory(history.value.id); loadError.value = '' } catch(e) { loadError.value = e?.message || String(e) } }
async function act(work) { busy.value = true; error.value = ''; try { await work(); await load() } catch(e) { error.value = e?.message || String(e) } finally { busy.value = false } }
function edit(job = newJob()) { generated.value = false; bodyText.value = JSON.stringify(job.action.body ?? null, null, 2); matchResult.value = null; editor.value = JSON.parse(JSON.stringify({ ...job, policy: { ...newJob().policy, ...job.policy } })); editor.value.enabled = Boolean(job.enabled); hookUrl.value = ''; triggerKind.value = job.triggerKind || (job.trigger.platform ? "message" : "schedule"); const t = job.trigger; scheduleType.value = t.every_minutes ? 'every_minutes' : t.at ? 'at' : 'cron'; scheduleValue.value = String(t[scheduleType.value] || ''); testRun.value = null; actionKind.value = job.action.kind === 'http' ? 'http' : job.action.kind === 'castle' ? 'castle' : job.action.kind === 'command' ? 'command' : 'companion'; argsText.value = JSON.stringify(job.action.args || [], null, 2); envText.value = JSON.stringify(job.action.env || {}, null, 2) }
async function save() { await act(async () => { const action = actionKind.value === 'command' ? { ...editor.value.action, ...commandFields(argsText.value, envText.value) } : actionKind.value === 'http' ? { ...editor.value.action, body: requestBody(editor.value.action.method, bodyText.value) } : editor.value.action; const job = { ...editor.value, approveProposal: editor.value.state === 'proposed', action, triggerKind: triggerKind.value, trigger: savedTrigger(triggerKind.value, editor.value.trigger, scheduleType.value, scheduleValue.value) }; await automationSave(job); editor.value = null }) }
async function describe() { await act(async () => { const draft = await automationDescribe(description.value); describeOpen.value = false; edit(draft); generated.value = true }) }
async function testMatch() { matchResult.value = null; await act(async () => { matchResult.value = await automationMessageMatch(editor.value.trigger, sampleText.value) }) }
async function showHistory(job) { history.value = job; await load() }
async function test() { await act(async () => { testRun.value = await automationRun(editor.value.id, true); history.value = { ...editor.value } }) }
function changeAction(kind) {
  actionKind.value = kind
  editor.value.action = kind === 'http' ? {kind:'http',method:'GET',url:'',body:null,header:'',key:''} : kind === 'castle' ? castleAction() : kind === 'command' ? { kind: 'command', program: '', cwd: '', args: [], env: {}, stdin: 'none' } : newJob().action
  if (['castle','http'].includes(kind)) editor.value.policy.retries = 0
  argsText.value = '[]'; envText.value = '{}'
}
async function pickOutputFolder() { await act(async () => { const folder = await chooseFolder('Choose a private output folder outside active memory'); if (folder) editor.value.action.folder = folder }) }
async function pickWorkingFolder() { await act(async () => { const folder = await chooseFolder('Choose the job working folder'); if (folder) editor.value.action.cwd = folder }) }
async function pickWatchFolder() { await act(async () => { const folder = await chooseFolder('Choose the folder to watch'); if (folder) editor.value.trigger.folder = folder }) }
async function makeLink() { await act(async () => { hookUrl.value = await automationHookReset(editor.value.id); linkCopied.value = false }) }
async function copyLink() { try { await navigator.clipboard.writeText(hookUrl.value); linkCopied.value = true } catch { error.value = 'Copying failed. Select the link and copy it yourself.' } }
function duplicate(job) { edit({ ...job, id: undefined, state: 'active', enabled: false, name: job.name.slice(0, 70) + ' copy' }) }
onMounted(() => { load(); timer = setInterval(load, 3000) }); onBeforeUnmount(() => clearInterval(timer))
</script>
<template>
<section class="automation-room">
 <header class="bar">
  <h2>AUTOMATIONS</h2><span class="spacer"></span>
  <label class="pause">Pause all <SettingSwitch :model-value="automations?.paused" label="Pause all automations" :disabled="busy" @update:model-value="v => act(() => automationPause(v))" /></label>
  <button @click="archived = !archived">{{ archived ? 'Active jobs' : 'Archived jobs' }}</button>
  <button @click="emit('settings', 'automations#programs')">Approved programs</button>
  <button :disabled="busy" @click="describeOpen = true">Describe what you want</button>
 <button :disabled="busy" @click="edit()">New job</button>
 </header>
 <p class="lede">Run scheduled jobs or react to approved messages while Castle stays running, including when hidden or minimized.</p>
 <RunnerHealth :health="automations?.health" />
 <p v-if="(error || loadError) && !editor" class="error" role="alert">{{ error || loadError }}</p>
 <p v-if="automations?.paused" role="status">New runs are paused. Missed schedules use their policy when you resume; incoming messages continue as ordinary chat.</p>
 <div v-if="!error && !loadError && !visibleJobs.length && !archived" class="empty castle-room-card"><h3>A little help at the right time</h3><p>Add a morning briefing or a weekly project review. New jobs start switched off.</p><button @click="edit()">Morning briefing</button> <button @click="edit({ ...newJob(), name: 'Weekly project review', trigger: { cron: '0 17 * * 5' }, action: { mode: 'think', prompt: 'Review my active projects in five lines. {{projects}} {{today}}' } })">Weekly project review</button></div>
 <article v-for="job in visibleJobs" :key="job.id" class="job castle-room-card" :class="{ live: Boolean(job.enabled) }">
  <SettingSwitch :model-value="Boolean(job.enabled)" :label="`Run ${job.name} on schedule`" :disabled="busy || archived || job.state === 'proposed'" @update:model-value="v => act(() => automationSave({ ...job, enabled: v }))" />
  <div class="job-main"><h3>{{ job.name }}</h3><p>{{ triggerSummary(job.trigger, job.triggerKind) }}</p><small>{{ actionSummary(job.action) }}</small><p>{{ listeningText(job) }}</p><small>{{ job.note }}</small><p v-if="job.state === 'proposed'">Proposal waiting for your approval</p></div>
  <span class="pill" :class="job.lastStatus">{{ pill(job.lastStatus) }}</span>
  <div class="job-actions"><button :disabled="busy || archived || job.state === 'proposed' || automations?.paused" @click="act(() => automationRun(job.id))">Run now</button><button :disabled="archived" @click="edit(job)">Edit</button><button @click="duplicate(job)">Duplicate</button><button @click="showHistory(job)">History</button><button class="danger" :disabled="busy || archived" @click="act(() => automationArchive(job.id))">Archive</button></div>
 </article>
 <p v-if="automations?.messageProblem" class="error" role="alert">{{ automations.messageProblem }}</p>
 <p v-if="automations?.watchProblem" class="error" role="alert">{{ automations.watchProblem }}</p>
 <p v-if="automations?.hookProblem" class="error" role="alert">{{ automations.hookProblem }}</p>
 <p v-if="automations?.eventProblem" class="error" role="alert">{{ automations.eventProblem }}</p>
 <ModalSheet v-if="describeOpen" tag="AUTOMATIONS" title="Describe a job" @close="describeOpen = false"><p>The companion drafts a job with no tools. Review it before saving; nothing runs from this description.</p><label>Description <textarea aria-label="Job description" v-model="description" rows="5" maxlength="8000" /></label><p v-if="error" class="error" role="alert">{{ error }}</p><button class="primary" :disabled="busy || !description.trim()" @click="describe">{{ busy ? 'Drafting...' : 'Draft job' }}</button></ModalSheet>
 <ModalSheet v-if="editor" tag="AUTOMATIONS" :title="editor.id ? 'Edit job' : 'New job'" @close="editor = null">
  <form class="job-editor castle-room-card" @submit.prevent="save">
   <p v-if="error || loadError" class="error" role="alert">{{ error || loadError }}</p>
   <p v-if="generated" role="status">Companion draft: review every field. This job is unsaved and switched off.</p><p v-if="editor.state === 'proposed'" role="status">Review this proposal. Approval saves it switched off; you choose when to enable it.</p>
   <label>Name <HelpTip text="A short, unique name for this job."/><input aria-label="Name" v-model="editor.name" maxlength="80" required /></label>
   <label>What it is for <HelpTip text="Your own note about why this job exists."/><input aria-label="What it is for" v-model="editor.note" /></label>
   <h3>Trigger</h3>
   <label>When to run <GlassSelect aria-label="Trigger type" :model-value="triggerKind" :options="[{value:'schedule',label:'On a schedule'},{value:'message',label:'When a message arrives'},{value:'file',label:'When a file appears'},{value:'webhook',label:'When a web request arrives'},{value:'event',label:'When something happens in Castle'}]" @update:model-value="value => { triggerKind = value; editor.trigger = triggerDefaults(value); hookUrl = '' }" /></label>
   <template v-if="triggerKind === 'message'">
    <label>Platform <GlassSelect aria-label="Message platform" v-model="editor.trigger.platform" :options="[{value:'any',label:'Any approved chat'},{value:'telegram',label:'Telegram'},{value:'discord',label:'Discord'},{value:'whatsapp',label:'WhatsApp'}]" /></label>
    <label>Match <GlassSelect aria-label="Message match" v-model="editor.trigger.match" :options="[{value:'contains',label:'Contains'},{value:'starts_with',label:'Starts with'},{value:'regex',label:'Regular expression'}]" /></label>
    <label>Pattern <HelpTip text="Up to 256 bytes. Regex has a 50 ms matching budget and a 16 KB message limit; lookaround and backreferences are not supported."/><input aria-label="Message pattern" v-model="editor.trigger.text" maxlength="256" required /></label>
    <label class="inline">Match case <SettingSwitch label="Match case" v-model="editor.trigger.case" /></label>
    <label class="inline">Also chat <HelpTip text="Normally a matched message only runs its jobs. Switch on to also send it to your companion's real chat. If several jobs match, every one must allow chat. Unmatched messages chat normally."/><SettingSwitch label="Also chat" v-model="editor.trigger.also_chat" /></label>
    <label>Sample message <textarea aria-label="Sample message" v-model="sampleText" rows="2" /></label><button type="button" :disabled="busy" @click="testMatch">Test match</button><p v-if="matchResult !== null" role="status">{{ matchResult ? 'Pattern matches the sample.' : 'Pattern does not match the sample.' }} No job or chat was started.</p>
    <p>Only the chat approved in MESSAGING can trigger a job. Automation actions treat message text as data. Session actions are unavailable for message jobs.</p>
   </template>
   <template v-else-if="triggerKind === 'file'">
    <label>Folder <HelpTip text="A folder on this computer. Files already there when the job is switched on are ignored. Network folders are not supported."/><input aria-label="Watched folder" v-model="editor.trigger.folder" required /><button type="button" :disabled="busy" @click="pickWatchFolder">Choose folder</button></label>
    <label>File names <HelpTip text="* matches any run of characters and ? matches one, for example *.pdf. Not case sensitive. Only files directly in the folder count."/><input aria-label="File name pattern" v-model="editor.trigger.glob" maxlength="64" required /></label>
    <label>Run when a file <GlassSelect aria-label="File event" v-model="editor.trigger.event" :options="[{value:'created',label:'appears'},{value:'changed',label:'appears or changes'}]" /></label>
    <label>Seconds to wait <HelpTip text="How long a file must stay unchanged before the job runs, so a file still being copied is not used half written."/><input aria-label="Seconds to wait" v-model.number="editor.trigger.settle_s" type="number" min="1" max="300" required /></label>
    <p>The file's name and path reach the job as plain text. Session actions are unavailable for file jobs, and a file that arrives while Castle is closed does not run.</p>
   </template>
   <template v-else-if="triggerKind === 'event'">
    <label>Event <GlassSelect aria-label="Castle event" v-model="editor.trigger.event" :options="EVENTS" /></label>
    <p v-if="editor.trigger.event === 'calendar_event'">This job starts only for the calendar events you link to it: open an event in CALENDAR and choose this job under "Start an automation". It does not start for every event.</p>
    <p>Castle's own events are trusted, so these jobs may use companion session actions. The event text reaches the job as plain text. Events that happen while Castle is closed do not run, and a job whose action would cause its own event is refused.</p>
   </template>
   <template v-else-if="triggerKind === 'webhook'">
    <p>Another program on this computer starts this job by sending a web request (POST) to its private link. The link works only from this computer and only while the job is switched on. The request body reaches the job as plain text.</p>
    <p v-if="!editor.id">Save the job first, then create its link here.</p>
    <template v-else>
     <p role="status">{{ automations?.hooks?.[editor.id] ? 'A link exists. Its secret is kept in your system secure store and cannot be shown again.' : 'There is no link yet. Create one, then switch the job on.' }}</p>
     <button type="button" :disabled="busy" @click="makeLink">{{ automations?.hooks?.[editor.id] ? 'Replace link' : 'Create link' }}</button>
     <div v-if="hookUrl" class="link"><input aria-label="Web request link" :value="hookUrl" readonly @focus="$event.target.select()" /><button type="button" @click="copyLink">{{ linkCopied ? 'Copied' : 'Copy' }}</button><p>Copy it now, it is shown only once. Replacing the link stops the old one.</p></div>
    </template>
   </template>
   <template v-else><label>Schedule <HelpTip text="Cron uses minute, hour, day, month and weekday. Restricted day and weekday fields match either. Local clock; missing times are skipped and repeated times run once."/><GlassSelect aria-label="Schedule type" v-model="scheduleType" :options="[{ value: 'cron', label: 'Calendar schedule' }, { value: 'every_minutes', label: 'Every few minutes' }, { value: 'at', label: 'Once at a date' }]" @change="scheduleValue = scheduleType === 'cron' ? '0 8 * * 1-5' : scheduleType === 'every_minutes' ? '30' : ''" /></label>
   <label>{{ scheduleType === 'cron' ? 'Cron expression' : scheduleType === 'at' ? 'Local date and time' : 'Minutes between runs' }}<input aria-label="Schedule value" v-model="scheduleValue" :type="scheduleType === 'at' ? 'datetime-local' : scheduleType === 'every_minutes' ? 'number' : 'text'" required min="1" /></label>
   <div v-if="scheduleType === 'cron'" class="presets"><button type="button" @click="scheduleValue = '0 8 * * 1-5'">Weekdays at 8</button><button type="button" @click="scheduleValue = '0 * * * *'">Every hour</button><button type="button" @click="scheduleValue = '0 20 * * 0'">Sunday at 8 pm</button></div>
   </template>
   <h3>Action</h3>
   <label>What to run <GlassSelect aria-label="Action type" :model-value="actionKind" :options="[{ value: 'companion', label: 'Companion briefing' }, { value: 'command', label: 'Approved program' }, { value: 'castle', label: 'Castle action' }, {value:'http',label:'HTTP request'}]" @update:model-value="changeAction" /></label>
   <template v-if="actionKind === 'companion'">
     <p>Ask your companion for a private briefing in an isolated run.</p>
     <label>Prompt <HelpTip text="Available context: {{work_log}}, {{reminders}}, {{projects}}, {{today}}, {{payload.text}}. This run follows your provider and has no access to your main session."/><textarea aria-label="Prompt" v-model="editor.action.prompt" rows="6" required /></label>
   </template>
   <template v-else-if="actionKind === 'command'">
     <label>Program <HelpTip text="Programs must be approved in Settings before a job can use them."/><GlassSelect aria-label="Program" v-model="editor.action.program" :options="programOptions" placeholder="Choose an approved program" /></label>
     <p v-if="!programOptions.length">Add a program in Settings > Automations first.</p>
     <label>Arguments <HelpTip text='A JSON list, for example ["C:/scripts/report.py", "--daily"]. Each item is one argument; payload text is never inserted here.'/><textarea aria-label="Arguments" v-model="argsText" rows="4" spellcheck="false" /></label>
     <label>Working folder <HelpTip text="The existing folder the program starts in."/><input aria-label="Working folder" v-model="editor.action.cwd" required /><button type="button" :disabled="busy" @click="pickWorkingFolder">Choose folder</button></label>
     <label>Environment <HelpTip text='Optional text values as a JSON object, for example {"REPORT_MODE":"daily"}. Keep secrets in your script protected configuration.'/><textarea aria-label="Environment" v-model="envText" rows="3" spellcheck="false" /></label>
     <label>Standard input <GlassSelect aria-label="Standard input" v-model="editor.action.stdin" :options="[{ value: 'none', label: 'No stdin' }, { value: 'payload', label: 'Payload as JSON' }]" /></label>
     <p>The payload is always available as AUTOMATION_PAYLOAD. Runs are hidden; time limits stop the whole process group. Exit codes and captured output appear in history.</p>
   </template>
   <template v-else-if="actionKind === 'http'">
    <label>Method <GlassSelect aria-label="HTTP method" v-model="editor.action.method" :options="['GET','POST']" /></label>
    <label>Address <HelpTip text="A fixed HTTPS address, or HTTP on 127.0.0.1/::1. No payload tokens, credentials or redirects."/><input aria-label="HTTP address" v-model="editor.action.url" required maxlength="2048" /></label>
    <label v-if="editor.action.method === 'POST'">JSON body <HelpTip text="Only JSON string values may contain {{payload.text}}. Message text is escaped as data; it cannot change JSON keys, the address or headers."/><textarea aria-label="HTTP JSON body" v-model="bodyText" rows="5" /></label>
    <label>Authentication <GlassSelect aria-label="HTTP authentication" v-model="editor.action.key" :options="[{value:'',label:'No authentication'},{value:'automation-http',label:'Automation HTTP key from Settings'}]" /></label>
    <label v-if="editor.action.key">Header name <HelpTip text="For example Authorization or X-API-Key. Save the complete header value in Settings > Keys > Automation HTTP authentication."/><input aria-label="HTTP header" v-model="editor.action.header" maxlength="80" required /></label><p>Test saved job validates without making a request. HTTP actions never retry automatically.</p>
   </template>
   <template v-else>
     <label>Castle action <GlassSelect aria-label="Castle action" :model-value="editor.action.action" :options="CASTLE_ACTIONS" @update:model-value="value => editor.action = castleAction(value)" /></label>
     <template v-if="['backup_memory', 'export_markdown'].includes(editor.action.action)">
       <label>Output folder <HelpTip text="Choose a private folder outside active memory. Each run creates a new subfolder. Backups stay local; no Git commit or push."/><input aria-label="Output folder" v-model="editor.action.folder" required /><button type="button" :disabled="busy" @click="pickOutputFolder">Choose output folder</button></label>
     </template>
     <template v-if="['add_reminder', 'add_task'].includes(editor.action.action)">
       <label>Title <HelpTip text="Use {{payload.text}} to insert the incoming message as plain text. Titles are limited to 255 characters. Project and due date stay fixed."/><input aria-label="Item title" v-model="editor.action.title" maxlength="255" required /></label>
       <label>Due date <input aria-label="Due date" v-model="editor.action.due" type="date" /></label>
     </template>
     <label v-if="editor.action.action === 'add_task'">Project <GlassSelect aria-label="Project" v-model="editor.action.project" :options="automations?.projects || []" placeholder="Choose an active project" /></label>
     <label v-if="['notify', 'add_reminder'].includes(editor.action.action)">{{ editor.action.action === 'notify' ? 'Notification' : 'Details' }} <textarea aria-label="Action text" v-model="editor.action.text" rows="3" :required="editor.action.action === 'notify'" /></label>
     <label v-if="editor.action.action === 'save_diary_prompt'" class="inline">Allow companion tools <HelpTip text="This sends the fixed request save diary into your real companion session. It may use memory and tools. Only trusted schedules, manual runs and Castle events are allowed."/><SettingSwitch :model-value="editor.action.allow_tools" label="Allow companion tools" @update:model-value="value => editor.action.allow_tools = value" /></label>
     <p>Test runs validate this action without changing Castle or contacting your companion. Message previews use the text "Sample invoice message".</p>
   </template>
   <h3>Delivery</h3>
   <div class="delivery-heading">When it succeeds <HelpTip text="Phone delivery goes only to the chat already approved in MESSAGING. At most six message parts per platform each hour; long results are split."/></div>
   <div class="delivery-options"><label v-for="destination in DESTINATIONS" :key="destination.kind" class="inline">{{ destination.label }}<SettingSwitch :label="`Success to ${destination.label}`" :model-value="editor.delivery.some(d => d.kind === destination.kind)" @update:model-value="value => editor.delivery = toggleDestination(editor.delivery, destination.kind, value)" /><small v-if="destination.kind !== 'home' && !automations?.phone?.find(p => p.kind === destination.kind)?.ready">Finish setup in MESSAGING</small></label></div>
   <div class="delivery-heading">If it fails <HelpTip text="Failures always appear on HOME. Choose any additional approved phone chats here. Delivery failures also appear on HOME."/></div>
   <div class="delivery-options"><label v-for="destination in DESTINATIONS" :key="destination.kind" class="inline">{{ destination.label }}<SettingSwitch :label="`Failure to ${destination.label}`" :disabled="destination.kind === 'home'" :model-value="destination.kind === 'home' || (editor.policy.on_failure_delivery || []).some(d => d.kind === destination.kind)" @update:model-value="value => editor.policy.on_failure_delivery = toggleDestination(editor.policy.on_failure_delivery, destination.kind, value)" /></label></div>
   <h3>Follow-up</h3><label>When it succeeds <GlassSelect aria-label="Success follow-up" :model-value="editor.policy.on_success_run || ''" @update:model-value="value => editor.policy.on_success_run = value" :options="followOptions" /></label><label>If it fails <GlassSelect aria-label="Failure follow-up" :model-value="editor.policy.on_failure_run || ''" @update:model-value="value => editor.policy.on_failure_run = value" :options="followOptions" /></label><p>Follow-ups must be approved and switched on. At most three steps after the first job; a job cannot repeat. Test runs never start follow-ups.</p>
   <h3>Safety</h3><label>Time limit in seconds <HelpTip text="Programs and isolated briefings stop at this limit. Castle actions never retry automatically; completed changes stay recorded if they overrun."/><input aria-label="Time limit in seconds" v-model.number="editor.policy.timeout_s" type="number" min="1" max="3600" /></label>
   <label v-if="triggerKind === 'schedule'">Missed while Castle was closed <GlassSelect aria-label="Missed schedules" v-model="editor.policy.missed" :options="[{ value: 'skip', label: 'Skip' }, { value: 'run_once', label: 'Run once on opening' }]" /></label>
   <p>At most two jobs run at once. Jobs never overlap themselves. New jobs start switched off.</p>
   <div class="buttons"><button class="primary" type="submit" :disabled="busy">{{ editor.state === 'proposed' ? 'Approve job' : 'Save job' }}</button><button type="button" :disabled="busy || !editor.id || editor.state === 'proposed' || automations?.paused" @click="test">Test saved job</button><button type="button" @click="editor = null">Cancel</button></div><p v-if="testRun">Test run {{ testRun }} recorded. Close the editor to see its history.</p>
  </form>
 </ModalSheet>
 <ModalSheet v-if="history && !editor" tag="RUN HISTORY" :title="history.name" @close="history = null">
  <p v-if="!runs.length">No runs yet.</p><article v-for="run in runs" :key="run.id" class="history castle-room-card"><strong>{{ pill(runStatus(run)) }}</strong> <time>{{ run.startedAt }}</time><p>{{ run.summary }}</p><small>{{ run.cause }} - Finished: {{ run.finishedAt || 'Still running' }}</small><details><summary>Output and delivery</summary><pre>{{ run.output || run.error || 'Waiting for output' }}</pre><ul class="delivery-results"><li v-for="(delivery, i) in run.delivered || []" :key="i">{{ DESTINATIONS.find(d => d.kind === delivery.kind)?.label || delivery.kind }}: {{ pill(delivery.status) }}<span v-if="delivery.parts"> ({{ delivery.parts }} parts)</span><span v-if="delivery.runId"> (run {{ delivery.runId }})</span><p v-if="delivery.error" class="error">{{ delivery.error }}</p></li></ul></details></article>
 </ModalSheet>
</section>
</template>
<style scoped>
.automation-room {
  position: absolute;
  inset: 24px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow: auto;
  color: var(--text);
  font-size: 13px;
  scrollbar-width: var(--scrollbar-width);
  scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track);
}
.bar, .pause, .inline { display: flex; align-items: center; gap: 12px; }
.bar { flex-wrap: wrap; }
.bar h2 { margin: 0; font-size: 13px; font-weight: 500; letter-spacing: 0.2em; color: var(--accent); }
.spacer { flex: 1; }
.lede { margin: 0; max-width: 60em; color: var(--text-dim); }
p { color: var(--text-dim); line-height: 1.5; }
h3 { margin: 0 0 8px; font-size: 17px; font-weight: 600; letter-spacing: 0.04em; }
.job, .empty { flex-shrink: 0; }
.job { display: flex; align-items: center; gap: 16px; flex-wrap: wrap; }
.job-main { flex: 1; min-width: 200px; }
.job p { margin: 4px 0; }
.job-actions, .buttons, .presets { display: flex; gap: 8px; flex-wrap: wrap; }
.pill { border: 1px solid var(--line); padding: 5px 10px; border-radius: 20px; font-size: 12px; }
.failed, .timed_out, .error { color: #ef9393; }
.job-editor { display: grid; gap: 16px; }
.job-editor h3 { margin: 0; font-size: 11px; font-weight: 500; letter-spacing: 0.16em; color: var(--accent); }
.job-editor p { margin: 0; }
.job-editor label { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }
.job-editor input, .job-editor textarea {
  width: 100%; box-sizing: border-box; padding: 10px; color: var(--text);
  background: var(--panel-background); border: 1px solid var(--line); border-radius: 8px; font: inherit;
}
.link { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }
.link input { flex: 1; min-width: 240px; }
.job-editor input:focus-visible, .job-editor textarea:focus-visible,
button:focus-visible, summary:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
button:where(:not(.setting-switch):not(.glass-select)) {
  padding: 8px 14px; border: 1px solid var(--line); border-radius: 10px;
  background: none; color: var(--text); font: inherit; cursor: pointer;
}
button:where(:not(.setting-switch):not(.glass-select)):hover:not(:disabled) { border-color: color-mix(in srgb, var(--accent) 45%, var(--line)); }
button:disabled { opacity: 0.45; cursor: default; }
button.primary { background: color-mix(in srgb, var(--accent) 25%, transparent); }
button.danger { border-color: color-mix(in srgb, #ef9393 55%, var(--line)); color: #ef9393; }
.history { margin-bottom: 16px; }
.history time { color: var(--text-dim); }
pre { white-space: pre-wrap; overflow-wrap: anywhere; font: inherit; line-height: 1.6; }
summary { cursor: pointer; }
.delivery-heading { display: flex; align-items: center; gap: 8px; color: var(--text-dim); }
.delivery-options { display: flex; flex-wrap: wrap; align-items: flex-start; gap: 12px 32px; }
.delivery-options label { display: grid; grid-template-columns: auto auto; align-items: center; justify-content: start; gap: 4px 12px; }
.delivery-options small { grid-column: 1 / -1; color: var(--text-dim); font-size: 11px; }
.job-editor input[type="number"] { width: 9em; }
.delivery-results { padding-left: 20px; }
.delivery-results li { margin: 8px 0; }

</style>
