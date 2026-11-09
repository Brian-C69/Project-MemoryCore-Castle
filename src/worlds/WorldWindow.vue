<script setup>
// A WORLD window on one of the member's other screens: the whole page is that screen's world.
// Its settings arrive before the page loads (world.rs) and again, without a reload, whenever the
// member changes them in the WORLD room. The lead window (the lowest screen switched on) plays
// the music.
import { onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import { listen } from '@tauri-apps/api/event'
import WorldStage from './WorldStage.vue'
import { worldsMine } from '../api/castle.js'

const props = defineProps({ initial: { type: Object, required: true } })

const config = ref(props.initial.config ?? {})
const lead = ref(!!props.initial.lead)
const music = reactive({ enabled: !!props.initial.music, volume: props.initial.volume ?? 0.5 })
const visible = ref(true)
const mine = ref([])
// this window has no session of its own: the crystal simply lives here
const signals = reactive({ busy: false, asleep: false, sentAt: 0 })
const stops = []

function take(payload) {
  config.value = payload.config ?? config.value
  lead.value = !!payload.lead
  music.enabled = !!payload.music
  music.volume = payload.volume ?? music.volume
}

onMounted(async () => {
  stops.push(await listen('castle-world-config', (event) => take(event.payload)))
  stops.push(await listen('world-visible', (event) => (visible.value = !!event.payload)))
  mine.value = await worldsMine().catch(() => [])
})
onBeforeUnmount(() => stops.forEach((stop) => stop()))
</script>

<template>
  <div class="world-window">
    <WorldStage :config="config" :signals="signals" :mine="mine" :lead="lead" :music="music" :active="visible" />
  </div>
</template>

<style scoped>
.world-window {
  position: fixed;
  inset: 0;
  background: #040817;
}
</style>
