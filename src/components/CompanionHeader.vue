<script setup>
// The top of the companion panel: the portrait (their picture, or the glowing crystal) with a
// status dot on its lower right as in the Keep, and the name. Which app they think with is chosen
// in Settings > Companion > Thinks with; a switch under way is shown here until it is done.
// Below the name: today's mood (chips and one sentence), as in the Keep - written each morning
// from a random lean, changed only by the companion itself (mood.rs).
import { computed, onMounted, onBeforeUnmount, ref } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { moodToday, providerSwitchStatus, providerSwitchCancel } from '../api/castle.js'

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

// today's mood: read now, every minute (a change by the companion, a new day) and when one is made
const mood = ref(null)
let moodTimer
let unlisten
async function readMood() {
  try { mood.value = await moodToday() } catch { mood.value = null }
}
onMounted(async () => {
  readMood()
  moodTimer = setInterval(readMood, 60_000)
  try { unlisten = await listen('mood-changed', readMood) } catch {}
})
onBeforeUnmount(() => { clearInterval(moodTimer); unlisten?.() })
const current = computed(() => mood.value?.mood ?? null)
// when the companion changed its mood itself: when and why, on the sentence
const moodWhy = computed(() => {
  const m = current.value
  if (!m) return ''
  if (m.source === 'companion') return `${props.name} changed its mood at ${m.at}${m.why ? `: ${m.why}` : ''}`
  return `Today's mood, since ${m.at}`
})

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
      <div v-if="current" class="mood" :title="moodWhy">
        <p class="mood-label">Mood</p>
        <ul class="chips" :aria-label="`${name}'s mood today`">
          <li v-for="(w, i) in current.words" :key="w" :class="'c' + (i % 4)">{{ w }}</li>
        </ul>
        <p class="divider" aria-hidden="true"><span>&#x2665;</span></p>
        <p class="tone">{{ current.tone }}</p>
      </div>
      <p v-else-if="mood?.making" class="mood-making">{{ name }} is waking into today's mood…</p>
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
.mood { margin-top: 4px; text-align: center; }
.mood-label {
  margin: 0 0 4px;
  color: var(--accent);
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.22em;
  text-transform: uppercase;
}
.chips {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 5px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.chips li {
  padding: 1px 10px;
  border: 1px solid var(--chip, var(--accent));
  border-radius: 999px;
  background: color-mix(in srgb, var(--chip, var(--accent)) 26%, transparent);
  color: var(--text);
  font-size: 12px;
  line-height: 1.6;
}
.chips .c1 { --chip: color-mix(in srgb, var(--accent) 45%, #e86a9a); }
.chips .c2 { --chip: color-mix(in srgb, var(--accent) 45%, #4fc3b4); }
.chips .c3 { --chip: color-mix(in srgb, var(--accent) 55%, #b58cff); }
.divider {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 5px 0 3px;
  color: var(--accent);
  font-size: 10px;
}
.divider::before,
.divider::after {
  content: '';
  flex: 1;
  height: 1px;
  background: color-mix(in srgb, var(--accent) 45%, transparent);
}
.tone {
  margin: 0;
  color: var(--text);
  font-family: Georgia, 'Times New Roman', serif;
  font-size: 13px;
  font-style: italic;
  line-height: 1.4;
}
.mood-making { margin: 4px 0 0; color: var(--text-dim); font-size: 12px; font-style: italic; }
</style>
