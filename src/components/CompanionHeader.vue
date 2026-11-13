<script setup>
// The top of the companion panel: the portrait (their picture, or the glowing crystal), the
// name, and which app they think with - as in the Constellation design.
import { computed, onMounted, onBeforeUnmount, ref } from 'vue'
import { memoryFeatures, providersList, providerSwitch, providerSwitchStatus, providerSwitchCancel } from '../api/castle.js'
import { currentProvider, providerStatus, shownProvider } from './providerpicker.js'
import { needsSavePrompt, saveChoices } from './switchsave.js'
const providers = ref([])
const switching = ref({})
const problem = ref('')
let timer
async function discover() {
  try { providers.value = await providersList() } catch (e) { problem.value = String(e) }
}
// The picker always shows the provider in use; choosing the other one starts the switch.
const shown = computed(() => shownProvider(providers.value, props.cliLabel, switching.value.target))
// A switch hands the new provider only the saved memory and the recent dialogue, so with a session
// running Castle asks first. `asking` is the provider chosen, waiting for the member's answer.
const ENTER = String.fromCharCode(13) // the Enter key, as the quick-action buttons send it
const asking = ref('')
const choices = ref(saveChoices())
async function startSwitch(target) {
  try { switching.value = await providerSwitch(target); problem.value = '' } catch (e) { problem.value = String(e) }
}
async function select(event) {
  const target = event.target.value
  const current = currentProvider(providers.value, props.cliLabel)
  if (target && needsSavePrompt({ running: props.running, target, current })) {
    try { choices.value = saveChoices(await memoryFeatures()) } catch { choices.value = saveChoices() }
    asking.value = target
  } else if (target && target !== current) {
    await startSwitch(target)
  }
  // Until Castle confirms the switch, keep showing what is really running (or the switch in progress).
  event.target.value = shown.value
}
// Type the save into the session as if the member had clicked its button, then queue the switch:
// Castle waits for the session to finish that turn before it replaces it.
async function saveThenSwitch(choice) {
  const target = asking.value
  asking.value = ''
  try {
    await props.write(choice.command)
    await props.write(ENTER)
  } catch (e) {
    problem.value = String(e)
    return
  }
  await startSwitch(target)
}
async function switchWithoutSaving() {
  const target = asking.value
  asking.value = ''
  await startSwitch(target)
}
async function cancel() { await providerSwitchCancel(); switching.value = {} }
onMounted(() => { discover(); timer = setInterval(async () => { try { switching.value = await providerSwitchStatus() } catch {} }, 1000) })
onBeforeUnmount(() => clearInterval(timer))
const props = defineProps({
  name: { type: String, default: 'Your companion' },
  cliLabel: { type: String, default: 'Claude Code' },
  avatar: { type: String, default: null },
  // Types into the running session, and whether there is one (from the Log panel).
  write: { type: Function, default: async () => {} },
  running: { type: Boolean, default: false },
})
</script>

<template>
  <div class="companion-header">
    <div class="avatar">
      <img v-if="avatar" :src="avatar" alt="" />
      <span v-else class="crystal">◆</span>
    </div>
    <div class="who">
      <p class="name">{{ name }}</p>
      <select aria-label="Conversation provider" :value="shown" :disabled="!!switching.target" @change="select">
        <option v-if="!shown" value="">{{ cliLabel }}</option>
        <option v-for="p in providers" :key="p.cli" :value="p.cli" :disabled="!p.installed">{{ p.label }} — {{ providerStatus(p) }}</option>
      </select>
      <button @click="discover">Refresh providers</button>
      <div v-if="asking" class="ask" role="group" aria-label="Save before switching">
        <p>Save first? {{ providers.find((p) => p.cli === asking)?.label ?? 'The other provider' }} gets your saved memory and the recent chat, not what is only in this session.</p>
        <button v-for="c in choices" :key="c.command" @click="saveThenSwitch(c)">{{ c.label }}, then switch</button>
        <button @click="switchWithoutSaving">Switch without saving</button>
        <button @click="asking = ''">Cancel</button>
      </div>
      <p v-if="switching.message" role="status">{{ switching.message }}</p>
      <button v-if="switching.target" @click="cancel">Cancel switch</button>
      <p v-if="problem" role="alert">{{ problem }}</p>
    </div>
  </div>
</template>

<style scoped>
.companion-header {
  display: flex;
  gap: 18px;
  align-items: center;
  padding: 4px 6px 6px;
}
.avatar {
  display: grid;
  place-items: center;
  flex: none;
  width: 84px;
  height: 84px;
  overflow: hidden;
  border: 2px solid var(--accent);
  border-radius: 50%;
  background: radial-gradient(circle at 40% 35%, #2a55b8, #0a1638 70%);
  box-shadow: 0 0 22px rgba(120, 165, 255, 0.35);
}
.avatar img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.crystal {
  font-size: 38px;
  color: #cfe0ff;
  text-shadow: 0 0 16px #7aa6ff, 0 0 4px #ffffff;
}
.who { min-width: 0; flex: 1; }
select { display: block; max-width: 100%; padding: 6px 10px; border: 1px solid var(--line); border-radius: 8px; background: var(--panel-background); color: var(--text); }
.ask { margin-top: 8px; padding: 8px 10px; border: 1px solid var(--line); border-radius: 8px; background: var(--hover); font-size: 13px; }
.ask p { margin: 0 0 4px; }
.ask button { display: block; width: 100%; text-align: left; }
button { margin-top: 6px; padding: 5px 8px; border: 1px solid var(--line); border-radius: 6px; background: var(--hover); color: var(--text); cursor: pointer; }
.name {
  margin: 0 0 8px;
  font-size: 22px;
  font-weight: 400;
}
.chip {
  padding: 4px 12px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: var(--hover);
  font-size: 13px;
}
</style>
