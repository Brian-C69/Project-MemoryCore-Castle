<script setup>
// Dev only (?stage=<look>): the stage with a sample world and one sample beat, to see every look
// in a browser without a story session.
import { onMounted } from 'vue'
import StageView from './StageView.vue'

const props = defineProps({ look: { type: String, default: 'classic' }, faces: { type: String, default: '' } })
const scene = '/src/world2d/worlds/isles-of-memory/03-memory-garden.jpg'
const world = {
  id: 'demo',
  name: 'The Shattered Skies',
  folder: '/',
  scenes: [scene],
  cast: props.faces ? [{ name: 'Nova', path: `${props.faces}/nova.png` }, { name: 'Haren', path: `${props.faces}/hero.png` }, { name: 'hero', path: `${props.faces}/hero.png` }] : [],
  music: [],
}
const SAMPLE = `## Chapter 3 - The Floating Bridge
[scene: memory garden]
The path leads to a narrow stone bridge, suspended between floating islands. The wind howls below, and the moonlight paints the void.

Nova: "Stay close - the bridge will not hold us both."
Haren: "Then I go first."

1. Cross now
2. Find another way
3. (Say something)`

onMounted(() => {
  setTimeout(() => window.dispatchEvent(new CustomEvent('castle-story-reply', { detail: SAMPLE })), 300)
  // ?advance=N clicks through N times, to see the later lines and the choices
  const clicks = Number(new URLSearchParams(location.search).get('advance') || 0)
  for (let i = 0; i < clicks; i++) setTimeout(() => document.querySelector('.look')?.click(), 2600 + i * 700)
})
</script>

<template>
  <div style="position: fixed; inset: 0; background: #050a1c">
    <StageView :world="world" companion="Nova" :look="look" :say="(t) => console.log('say', t)" />
  </div>
</template>
