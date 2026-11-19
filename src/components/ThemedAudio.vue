<script setup>
// A small audio player in Castle's own colours (the browser's own is a white bar that ignores the
// theme): play / pause, a position slider, the time, speed, mute and volume. Used for every voice
// line and song in the app. One clip plays at a time, and the volume is shared by all players.
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { playerClock } from './playerclock.js'
import { nextRate, playerPrefs, rateLabel, setVolume, sharedPlayback, toggleMute } from './playerstate.js'

const props = defineProps({ src: { type: String, required: true }, compact: { type: Boolean, default: false } })

let uid = 0
const id = `player-${++uid}-${Math.random().toString(36).slice(2)}`
const rate = ref(1)

const audio = ref(null)
const playing = ref(false)
const position = ref(0)
const length = ref(0)
let dragging = false

const toggle = () => {
  const el = audio.value
  if (!el) return
  if (el.paused) el.play().catch(() => {})
  else el.pause()
}
const onTime = () => {
  if (!dragging && audio.value) position.value = audio.value.currentTime
}
const onMeta = () => {
  length.value = audio.value && Number.isFinite(audio.value.duration) ? audio.value.duration : 0
}
const onEnded = () => {
  playing.value = false
  position.value = 0
  sharedPlayback.release(id)
}
const onPlay = () => {
  playing.value = true
  sharedPlayback.claim(id, () => audio.value?.pause())
}
const onPause = () => {
  playing.value = false
}
const cycleRate = () => {
  rate.value = nextRate(rate.value)
  if (audio.value) audio.value.playbackRate = rate.value
}
const applyVolume = () => {
  if (!audio.value) return
  audio.value.volume = playerPrefs.volume
  audio.value.muted = playerPrefs.muted
}
const onVolume = (event) => setVolume(Number(event.target.value))
onMounted(applyVolume)
watch(() => [playerPrefs.volume, playerPrefs.muted], applyVolume)
const seek = (event) => {
  position.value = Number(event.target.value)
  if (audio.value) audio.value.currentTime = position.value
}

// a new clip starts from the top
watch(
  () => props.src,
  () => {
    position.value = 0
    length.value = 0
    playing.value = false
    rate.value = 1
  },
)
onBeforeUnmount(() => {
  audio.value?.pause()
  sharedPlayback.release(id)
})
</script>

<template>
  <div class="player">
    <audio
      ref="audio"
      :src="src"
      preload="metadata"
      @timeupdate="onTime"
      @loadedmetadata="onMeta"
      @durationchange="onMeta"
      @play="onPlay"
      @pause="onPause"
      @ended="onEnded"
    ></audio>
    <button class="play" :aria-label="playing ? 'Pause' : 'Play'" @click="toggle">{{ playing ? '❚❚' : '▶' }}</button>
    <input
      class="slider"
      type="range"
      min="0"
      :max="length || 1"
      step="0.05"
      :value="position"
      :disabled="!length"
      aria-label="Position"
      @pointerdown="dragging = true"
      @pointerup="dragging = false"
      @input="seek"
    />
    <span class="time">{{ playerClock(position) }} / {{ playerClock(length) }}</span>
    <button class="rate" :aria-label="`Speed ${rateLabel(rate)}`" title="Speed" @click="cycleRate">{{ rateLabel(rate) }}</button>
    <button class="mute" :aria-label="playerPrefs.muted ? 'Unmute' : 'Mute'" :title="playerPrefs.muted ? 'Unmute' : 'Mute'" @click="toggleMute">{{ playerPrefs.muted || playerPrefs.volume === 0 ? '🔇' : playerPrefs.volume < 0.5 ? '🔉' : '🔊' }}</button>
    <input
      v-if="!compact"
      class="slider volume"
      type="range"
      min="0"
      max="1"
      step="0.05"
      :value="playerPrefs.muted ? 0 : playerPrefs.volume"
      aria-label="Volume"
      @input="onVolume"
    />
  </div>
</template>

<style scoped>
.player {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 12px 6px 6px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: color-mix(in srgb, var(--accent) 8%, transparent);
  color: var(--text);
}
.play {
  display: grid;
  place-items: center;
  flex-shrink: 0;
  width: 30px;
  height: 30px;
  padding: 0;
  border: 1px solid var(--accent);
  border-radius: 50%;
  background: color-mix(in srgb, var(--accent) 18%, transparent);
  color: var(--accent);
  font-size: 11px;
  cursor: pointer;
}
.play:hover {
  background: color-mix(in srgb, var(--accent) 30%, transparent);
}
.slider {
  flex: 1;
  min-width: 0;
  height: 4px;
  accent-color: var(--accent);
  cursor: pointer;
}
.slider:disabled {
  cursor: default;
  opacity: 0.5;
}
.volume {
  flex: 0 0 64px;
}
.rate,
.mute {
  flex-shrink: 0;
  padding: 2px 6px;
  border: 1px solid transparent;
  border-radius: 999px;
  background: transparent;
  color: var(--text-dim);
  font-size: 12px;
  cursor: pointer;
}
.rate:hover,
.mute:hover {
  border-color: var(--line);
  color: var(--text);
}
.time {
  flex-shrink: 0;
  font-size: 12px;
  font-variant-numeric: tabular-nums;
  color: var(--text-dim);
}
</style>
