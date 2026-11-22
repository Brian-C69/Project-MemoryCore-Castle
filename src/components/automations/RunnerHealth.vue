<script setup>
import { computed, ref, onMounted, onBeforeUnmount } from 'vue'
import { runnerStatus, heartbeatTime } from './health.js'
const props = defineProps({ health: { type: Object, default: null }, compact: Boolean })
const now = ref(Date.now())
let timer
onMounted(() => { timer = setInterval(() => { now.value = Date.now() }, 5000) })
onBeforeUnmount(() => clearInterval(timer))
const status = computed(() => runnerStatus(props.health, now.value))
</script>
<template>
 <div class="runner-health" :class="status.tone" role="status">
  <strong>{{ status.label }}</strong><p>{{ status.detail }}</p>
  <details v-if="health && !compact"><summary>Runner details</summary><dl>
   <dt>Last heartbeat</dt><dd>{{ heartbeatTime(health.lastHeartbeat) }}</dd>
   <dt>Last completed check</dt><dd>{{ heartbeatTime(health.lastScan) }}</dd>
   <dt>Active runs</dt><dd>{{ health.activeRuns ?? 'Checking' }}</dd>
   <dt>Results waiting to save</dt><dd>{{ health.pendingResults || 0 }}</dd>
   <dt>Start at sign-in</dt><dd>{{ health.startWithSystem ? 'On' : 'Off' }}</dd>
   <template v-if="health.lastInterruption"><dt>Last recovery</dt><dd>{{ health.lastInterruption }} · {{ heartbeatTime(health.lastInterruptionAt) }}</dd></template>
  </dl><p v-if="health.startupProblem">{{ health.startupProblem }}</p><p v-if="health.problem && health.mode !== 'attention'">{{ health.problem }}</p><p>Hiding or minimizing keeps Castle running. Quit stops all automations until Castle opens again.</p></details>
 </div>
</template>
<style scoped>
.runner-health { color: var(--text-dim); font-size: 12px; }
strong { color: var(--text); font-weight: 500; }
p { margin: 5px 0 0; line-height: 1.5; }
.bad strong { color: #ef9393; }
.ok strong { color: var(--accent); }
summary { margin-top: 8px; cursor: pointer; }
dl { display: grid; grid-template-columns: max-content 1fr; gap: 6px 16px; margin: 10px 0; }
dt,dd { margin: 0; overflow-wrap: anywhere; }
@media(max-width:600px) { dl { grid-template-columns: 1fr; gap: 4px; } dd { margin-bottom: 6px; } }
</style>
