<script setup>
// The top of the companion panel: the portrait (their picture, or the glowing crystal) with a
// status dot on its lower right as in the Keep, and the name. Which app they think with is chosen
// in Settings > Companion > Thinks with; a switch under way is shown here until it is done.
import { computed, onMounted, onBeforeUnmount, ref } from 'vue'
import { providerSwitchStatus, providerSwitchCancel } from '../api/castle.js'

const props = defineProps({
  name: { type: String, default: 'Your companion' },
  avatar: { type: String, default: null },
  // whether the session runs, and whether the companion is working on a reply (from the panel)
  running: { type: Boolean, default: false },
  busy: { type: Boolean, default: false },
})

const switching = ref({})
const problem = ref('')
let timer
async function cancel() {
  try { await providerSwitchCancel(); switching.value = {} } catch (e) { problem.value = String(e) }
}
onMounted(() => { timer = setInterval(async () => { try { switching.value = await providerSwitchStatus() } catch {} }, 1000) })
onBeforeUnmount(() => clearInterval(timer))

// No dot while resting; a green glow while here; a green pulse while working.
const online = computed(() => (!props.running ? null : props.busy ? 'working' : 'awake'))
const onlineWords = computed(() => (online.value === 'working' ? `${props.name} is working` : `${props.name} is here`))
</script>

<template>
  <div class="companion-header">
    <span class="avatar-wrap">
      <div class="avatar">
        <img v-if="avatar" :src="avatar" alt="" />
        <span v-else class="crystal">◆</span>
      </div>
      <span v-if="online" class="online" :class="online" role="status" :title="onlineWords" :aria-label="onlineWords"></span>
    </span>
    <div class="who">
      <p class="name">{{ name }}</p>
      <p v-if="switching.message" class="switching" role="status">{{ switching.message }}</p>
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
/* the frame clips its picture, so the dot lives on this wrapper - outside the picture */
.avatar-wrap {
  position: relative;
  flex: none;
  line-height: 0;
}
.avatar {
  display: grid;
  place-items: center;
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
.online {
  position: absolute;
  right: -1px;
  bottom: 5%;
  width: 16px;
  height: 16px;
  border: 3px solid var(--panel-background, #0a1638);
  border-radius: 50%;
  background: #4ade80;
  box-shadow: 0 0 10px 2px rgba(74, 222, 128, 0.75);
}
.online.working {
  animation: online-pulse 1.4s ease-in-out infinite;
}
@keyframes online-pulse {
  50% {
    box-shadow: 0 0 2px 0 rgba(74, 222, 128, 0.35);
    opacity: 0.7;
  }
}
@media (prefers-reduced-motion: reduce) {
  .online.working { animation: none; }
}
.who { min-width: 0; flex: 1; }
.switching { margin: 0; font-size: 13px; color: var(--text-dim); }
button { margin-top: 6px; padding: 5px 8px; border: 1px solid var(--line); border-radius: 6px; background: var(--hover); color: var(--text); cursor: pointer; }
.name {
  margin: 0;
  font-size: 22px;
  font-weight: 400;
}
</style>
