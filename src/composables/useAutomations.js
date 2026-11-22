import { ref } from 'vue'
import { automationList, automationHealth } from '../api/castle.js'
export const automations = ref(null)
export const runnerHealth = ref(null)
let pending = null
export function prepareAutomations() {
 if (pending) return pending
 pending = (async () => {
  const healthWork = automationHealth().then(health => {
   runnerHealth.value = health
   if (health) automations.value = { jobs: [], notices: [], ...automations.value, health, paused: health.mode === 'paused', on: health.mode !== 'off' }
  }).catch(() => {})
  try { const data = await automationList(); await healthWork; automations.value = { ...data, health: runnerHealth.value }; return automations.value }
  catch (error) { await healthWork; throw error }
 })().finally(() => { pending = null })
 return pending
}
