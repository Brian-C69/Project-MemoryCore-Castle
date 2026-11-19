<script setup>
// Settings > Sentinel: on and off, the model the companion judges on (the recommended one, or a
// name the member types), how often it patrols, the one line that sets it up (run by the member
// in a PowerShell opened as administrator - Castle never asks for admin rights itself), and the
// line that removes it.
import { computed, onMounted, ref } from 'vue'
import { sentinelSettingsSave, sentinelSetup } from '../api/castle.js'
import GlassSelect from '../components/GlassSelect.vue'
import HelpTip from '../components/HelpTip.vue'
import { MAC_LINE } from '../components/sentinel.js'

const props = defineProps({
  settings: { type: Object, required: true },
})
const emit = defineEmits(['saved'])

const setup = ref(null)
const busy = ref('')
const said = ref('')
const problem = ref('')
const gear = computed(() => props.settings.sentinel || { on: false, model: null, patrolMinutes: 60 })
const modelChoice = ref(gear.value.model ? 'own' : 'recommended')
const modelName = ref(gear.value.model || '')
const minutes = ref(gear.value.patrolMinutes || 60)
const companion = computed(() => props.settings.companionName ?? 'your companion')
const recommendedText = computed(() => (setup.value?.recommended ? `${setup.value.recommended} (recommended for ${setup.value.cliLabel})` : `${setup.value?.cliLabel ?? 'your CLI'}'s own model (recommended)`))
const PATROLS = [15, 30, 60, 120, 240, 720].map((m) => ({ value: m, label: m < 60 ? `${m} minutes` : m === 60 ? 'hour' : `${m / 60} hours` }))

async function read(message = '') {
  busy.value = 'Looking…'
  problem.value = ''
  try {
    setup.value = await sentinelSetup()
    said.value = message
  } catch (reason) {
    problem.value = String(reason)
  } finally {
    busy.value = ''
  }
}

async function save(patch, message) {
  busy.value = 'Saving…'
  problem.value = ''
  const next = { on: gear.value.on, model: gear.value.model, patrolMinutes: gear.value.patrolMinutes, ...patch }
  try {
    await sentinelSettingsSave(next.on, next.model, next.patrolMinutes)
    said.value = message
    emit('saved')
  } catch (reason) {
    problem.value = String(reason)
  } finally {
    busy.value = ''
  }
}

function chooseModel(choice) {
  modelChoice.value = choice
  if (choice === 'recommended') save({ model: null }, 'The recommended model it is.')
}

async function copy(text) {
  try {
    await navigator.clipboard.writeText(text)
    said.value = 'Copied. Paste it into a PowerShell opened as administrator.'
  } catch {
    problem.value = 'It could not be copied - select the line and copy it by hand.'
  }
}

onMounted(() => read())
</script>

<template>
  <div class="sentinel-settings">
    <p v-if="setup && !setup.supported" class="hint">{{ MAC_LINE }}</p>
    <template v-else>
      <h3 class="with-tip">Keep watch <HelpTip :text="`${companion} keeps watch over this computer: the antivirus, the firewall, what listens for connections, what starts with Windows, scheduled tasks, new services, accounts and sign-ins. A small watch runs every five minutes, even when Castle is closed, and switches the firewall and protection back on if they are found off. ${companion} looks over what is new on each patrol and says one sentence on the shield. Nothing about this computer leaves it.`" /></h3>
      <label class="toggle"><input type="checkbox" :checked="gear.on" :disabled="!!busy" @change="save({ on: $event.target.checked }, $event.target.checked ? 'The Sentinel is on.' : 'The Sentinel is off. The watch keeps running until you remove it.')" /> Let {{ companion }} keep watch</label>

      <h3 class="with-tip">The model <HelpTip :text="`Each patrol is one short run of ${companion} on your own plan, and only when something is open. The recommended model judges steadily at a fair cost; you can name any model your CLI knows.`" /></h3>
      <label class="toggle"><input type="radio" name="sentinel-model" :checked="modelChoice === 'recommended'" :disabled="!!busy" @change="chooseModel('recommended')" /> {{ recommendedText }}</label>
      <label class="toggle"><input type="radio" name="sentinel-model" :checked="modelChoice === 'own'" :disabled="!!busy" @change="chooseModel('own')" /> Another model</label>
      <div v-if="modelChoice === 'own'" class="row">
        <input v-model="modelName" class="model" placeholder="for example opus" maxlength="64" :disabled="!!busy" @keydown.enter="save({ model: modelName }, 'The model is changed.')" />
        <button :disabled="!!busy || !modelName.trim()" @click="save({ model: modelName }, 'The model is changed.')">Use it</button>
      </div>

      <h3 class="with-tip">How often <HelpTip text="An alarm always starts a patrol at once." /></h3>
      <label class="toggle">Patrol every
        <GlassSelect :model-value="minutes" :options="PATROLS" :disabled="!!busy" aria-label="Patrol every" @update:model-value="(m) => { minutes = m; save({ patrolMinutes: m }, 'Saved.') }" />
      </label>

      <h3 class="with-tip">Set it up <HelpTip text="The line installs a small helper where only an administrator can change it, and three scheduled tasks: the five-minute watch, a second look whenever the antivirus or the firewall raises an alarm, and the helper itself. The helper can only close this computer further - switch the firewall or protection back on, scan, update the antivirus, block an address that keeps knocking for a day - and, only when you press its button, block a port, switch off a task or a startup entry, or stop a program." /></h3>
      <p class="hint">{{ setup?.installed?.line ?? 'Looking…' }}</p>
      <template v-if="setup?.installLine">
        <p class="hint has-tip">Run this line in PowerShell as administrator.<HelpTip block text="Right-click PowerShell, choose Run as administrator, paste this line and press Enter. Castle never asks for administrator rights itself." /></p>
        <div class="line"><code>{{ setup.installLine }}</code></div>
        <div class="row">
          <button @click="copy(setup.installLine)">Copy the line</button>
          <button :disabled="!!busy" @click="read('Checked.')">I ran it, check</button>
        </div>
      </template>

      <h3 class="with-tip">Remove it <HelpTip text="The same way, this line removes the helper, its tasks, its folders and every firewall rule it made." /></h3>
      <template v-if="setup?.uninstallLine">
        <div class="line"><code>{{ setup.uninstallLine }}</code></div>
        <div class="row"><button class="quiet" @click="copy(setup.uninstallLine)">Copy the line</button></div>
      </template>
    </template>
    <p v-if="busy" class="hint">{{ busy }}</p>
    <p v-if="said" class="said">{{ said }}</p>
    <p v-if="problem" class="problem">{{ problem }}</p>
  </div>
</template>

<style scoped>
.sentinel-settings {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
h3.with-tip {
  display: flex;
  align-items: center;
}
.has-tip { position: relative; }
.row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
}
.model {
  min-width: 200px;
}
.line {
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--widget-background);
  font-size: 12px;
  overflow-wrap: anywhere;
  user-select: all;
}
.said {
  margin: 0;
  color: var(--good);
}
.problem {
  margin: 0;
  color: var(--bad);
}
</style>
