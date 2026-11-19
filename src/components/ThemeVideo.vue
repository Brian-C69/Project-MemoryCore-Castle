<script setup>
// A theme's living wallpaper as a short seamless video (theme.json "motionVideo"), laid over the
// still picture in the same cover box. Off (Settings > Rooms > Moving wallpapers), with reduced
// motion, on another tab or with the window hidden it rests; off, the file is never even loaded
// and HOME shows the still picture. It fades in on its first frame, so the switch is seamless.
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { shouldLoad, shouldPlay } from '../themes/videoloop.js'

const props = defineProps({
  theme: { type: Object, default: null },
  visible: { type: Boolean, default: true },
  enabled: { type: Boolean, default: false },
})

const video = ref(null)
const shown = ref(false)
const awake = ref(document.visibilityState === 'visible')
const query = window.matchMedia?.('(prefers-reduced-motion: reduce)')
const calm = ref(query?.matches ?? false)

const src = computed(() => (shouldLoad({ enabled: props.enabled, calm: calm.value }) ? props.theme?.motionVideoUrl ?? null : null))
const playing = computed(() => Boolean(src.value) && shouldPlay({ enabled: props.enabled, visible: props.visible, awake: awake.value, calm: calm.value }))
const position = computed(() => props.theme?.wallpaperPosition ?? 'center')

function sync() {
  const el = video.value
  if (!el) return
  if (playing.value) el.play().catch(() => {})
  else el.pause()
}

const onVisibility = () => (awake.value = document.visibilityState === 'visible')
const onCalm = (e) => (calm.value = e.matches)

watch(playing, sync)
watch(src, () => (shown.value = false))
onMounted(() => {
  document.addEventListener('visibilitychange', onVisibility)
  query?.addEventListener?.('change', onCalm)
  sync()
})
onBeforeUnmount(() => {
  document.removeEventListener('visibilitychange', onVisibility)
  query?.removeEventListener?.('change', onCalm)
})
</script>

<template>
  <video
    v-if="src"
    ref="video"
    class="theme-video"
    :class="{ shown }"
    :src="src"
    :style="{ objectPosition: position }"
    muted
    loop
    playsinline
    preload="auto"
    disablepictureinpicture
    aria-hidden="true"
    @playing="shown = true"
    @loadeddata="sync"
  ></video>
</template>

<style scoped>
.theme-video {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
  pointer-events: none;
  opacity: 0;
  transition: opacity 0.8s ease;
}
.theme-video.shown {
  opacity: 1;
}
</style>
