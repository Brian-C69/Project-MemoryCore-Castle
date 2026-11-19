<script setup>
// A theme's looping sound on HOME (theme.json "motionSound"): the shores' waves, the river's water
// and birds. Played through Web Audio - decoded once, looped by an AudioBufferSourceNode, so the
// join has no gap - under any music at a share of the music slider's level, with a short fade in
// and out. It is heard only while Wallpaper sounds is on, HOME is showing and the window is
// visible; with the switch off the file is never fetched.
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { getSettings } from '../api/castle.js'
import { musicFrom } from './music.js'
import { shouldSound, soundLevel } from '../themes/themesound.js'

const props = defineProps({
  theme: { type: Object, default: null },
  visible: { type: Boolean, default: true },
  enabled: { type: Boolean, default: false },
})

const FADE_S = 0.8
const awake = ref(document.visibilityState === 'visible')
const volume = ref(0.5)
const url = computed(() => (props.enabled ? props.theme?.motionSoundUrl ?? null : null))
const heard = computed(() => Boolean(url.value) && shouldSound({ on: props.enabled, visible: props.visible, awake: awake.value }))

let context = null
let gain = null
let source = null
let loadedUrl = null
let loading = null
let buffer = null

function ensureContext() {
  if (context) return context
  const Ctx = window.AudioContext || window.webkitAudioContext
  if (!Ctx) return null
  context = new Ctx()
  gain = context.createGain()
  gain.gain.value = 0
  gain.connect(context.destination)
  return context
}

async function load(target) {
  if (loadedUrl === target && buffer) return buffer
  if (loading?.url === target) return loading.promise
  const promise = (async () => {
    const answer = await fetch(target)
    const bytes = await answer.arrayBuffer()
    return await context.decodeAudioData(bytes)
  })()
  loading = { url: target, promise }
  try {
    const decoded = await promise
    if (loading?.url === target) {
      buffer = decoded
      loadedUrl = target
    }
    return decoded
  } finally {
    if (loading?.url === target) loading = null
  }
}

function fadeTo(level) {
  if (!gain || !context) return
  const now = context.currentTime
  gain.gain.cancelScheduledValues(now)
  gain.gain.setValueAtTime(gain.gain.value, now)
  gain.gain.linearRampToValueAtTime(level, now + FADE_S)
}

function stopSource() {
  if (!source) return
  const old = source
  source = null
  try {
    old.stop(context.currentTime + FADE_S)
  } catch {
    // already stopped
  }
}

async function sync() {
  if (!heard.value) {
    if (context && source) {
      fadeTo(0)
      setTimeout(() => {
        if (!heard.value && context?.state === 'running') context.suspend().catch(() => {})
      }, FADE_S * 1000 + 50)
    }
    return
  }
  if (!ensureContext()) return
  const target = url.value
  try {
    if (context.state === 'suspended') await context.resume().catch(() => {})
    const decoded = await load(target)
    if (!heard.value || url.value !== target) return
    if (!source || source.buffer !== decoded) {
      stopSource()
      const next = context.createBufferSource()
      next.buffer = decoded
      next.loop = true
      next.connect(gain)
      next.start()
      source = next
    }
    fadeTo(soundLevel(volume.value))
  } catch {
    // the sound could not be read: HOME stays quiet
  }
}

const onVisibility = () => (awake.value = document.visibilityState === 'visible')
// the top bar's music slider, followed at once
function onMusic(event) {
  const v = event.detail?.volume
  if (typeof v !== 'number') return
  volume.value = v
  if (heard.value && source) fadeTo(soundLevel(v))
}
// a context that started suspended (no click yet) begins on the first one
const onFirstPress = () => {
  if (heard.value && context?.state === 'suspended') sync()
}

async function readVolume() {
  try {
    const view = await getSettings()
    volume.value = musicFrom(view.settings ?? view).volume
  } catch {
    // outside Castle: the starting level
  }
}

watch(heard, sync)
watch(url, (now, before) => {
  if (now !== before) {
    stopSource()
    buffer = null
    loadedUrl = null
    sync()
  }
})
onMounted(async () => {
  document.addEventListener('visibilitychange', onVisibility)
  window.addEventListener('castle-music', onMusic)
  window.addEventListener('pointerdown', onFirstPress)
  await readVolume()
  sync()
})
onBeforeUnmount(() => {
  document.removeEventListener('visibilitychange', onVisibility)
  window.removeEventListener('castle-music', onMusic)
  window.removeEventListener('pointerdown', onFirstPress)
  stopSource()
  context?.close().catch(() => {})
  context = null
})
</script>

<template>
  <span v-show="false" aria-hidden="true"></span>
</template>
