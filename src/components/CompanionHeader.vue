<script setup>
// The top of the companion panel: the portrait (their picture, or the glowing crystal), the
// name, and which app they think with - as in the Constellation design.
import { onMounted, onBeforeUnmount, ref } from 'vue'
import { providersList, providerSwitch, providerSwitchStatus, providerSwitchCancel } from '../api/castle.js'
const providers = ref([])
const switching = ref({})
const problem = ref('')
let timer
async function discover() {
  try { providers.value = await providersList() } catch (e) { problem.value = String(e) }
}
async function select(event) {
  if (!event.target.value) return
  try { switching.value = await providerSwitch(event.target.value); problem.value = '' } catch (e) { problem.value = String(e) }
  event.target.value = ''
}
async function cancel() { await providerSwitchCancel(); switching.value = {} }
onMounted(() => { discover(); timer = setInterval(async () => { try { switching.value = await providerSwitchStatus() } catch {} }, 1000) })
onBeforeUnmount(() => clearInterval(timer))
defineProps({
  name: { type: String, default: 'Your companion' },
  cliLabel: { type: String, default: 'Claude Code' },
  avatar: { type: String, default: null },
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
      <select aria-label="Switch conversation provider" :disabled="!!switching.target" @change="select">
        <option value="">{{ cliLabel }} — switch provider…</option>
        <option v-for="p in providers" :key="p.cli" :value="p.cli" :disabled="!p.installed">{{ p.label }} — {{ !p.installed ? 'not installed' : p.signedIn ? 'signed in' : p.signedIn === false ? 'sign in needed' : 'status unknown' }}</option>
      </select>
      <button @click="discover">Refresh providers</button>
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
