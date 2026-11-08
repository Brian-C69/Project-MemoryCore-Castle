<script setup>
// A WORLD window on one of the member's other screens: the whole page is the 3D isles or the 2D
// slideshow. The mode arrives before the page loads (world.rs) and can change while it is open.
import { computed, defineAsyncComponent, onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import Slideshow from './Slideshow.vue'
import { studioLibrary } from '../api/castle.js'

const World3D = defineAsyncComponent(() => import('../world3d/World3D.vue'))

const props = defineProps({ initial: { type: Object, required: true } })
const mode = ref(props.initial.mode === '2d' ? '2d' : '3d')
const lib = ref({ images: [] })
// this window has no session of its own: the crystal simply lives here
const signals = reactive({ busy: false, asleep: false, sentAt: 0 })
const pictures = computed(() => lib.value.images.map((m) => convertFileSrc(m.path)))

function onMode(event) {
  mode.value = event.detail === '2d' ? '2d' : '3d'
}

onMounted(async () => {
  window.addEventListener('castle-world-mode', onMode)
  try {
    lib.value = await studioLibrary()
  } catch {
    // no pictures
  }
})
onBeforeUnmount(() => window.removeEventListener('castle-world-mode', onMode))
</script>

<template>
  <div class="world-window">
    <World3D v-if="mode === '3d'" :signals="signals" />
    <Slideshow v-else :pictures="pictures" :interval="15" />
  </div>
</template>

<style scoped>
.world-window {
  position: fixed;
  inset: 0;
  background: #040817;
}
</style>
