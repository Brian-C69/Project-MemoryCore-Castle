<script setup>
// Look E - JRPG window: the scene fills the screen; an ornate framed window at the bottom holds
// the speaker's big face on the left and the line on the right; choices as framed options below.
import { computed, ref } from 'vue'
import SayBox from './SayBox.vue'

const props = defineProps({ st: { type: Object, required: true }, world: { type: Object, required: true }, companion: { type: String, default: '' } })
const sayBox = ref(null)
const speaker = computed(() => props.st.current?.speaker ?? null)
const face = computed(() => (speaker.value ? props.st.faceOf(speaker.value) : null))
function pick(i) {
  if (props.st.choose(i) === 'say') sayBox.value?.focus()
}
</script>

<template>
  <div class="jrpg" :class="{ cinema: st.s.cinema }" @click="st.next()">
    <div class="scene" :style="st.s.scene ? { backgroundImage: `url(&quot;${st.s.scene}&quot;)` } : {}"></div>
    <div v-if="st.current && !st.s.clean" class="window">
      <div v-if="speaker && !st.s.cinema" class="face">
        <img v-if="face" :src="face" alt="" />
        <span v-else>{{ st.initial(speaker) }}</span>
      </div>
      <div class="words">
        <span v-if="speaker" class="name">{{ speaker.toUpperCase() }}</span>
        <p :class="{ narration: !speaker }">{{ st.s.typed }} <span v-if="!st.s.typing" class="tri">▼</span></p>
        <SayBox v-if="st.atEnd && !st.s.cinema" ref="sayBox" class="say" @say="st.speak($event)" />
      </div>
    </div>
    <div v-if="st.choices.length && !st.s.clean" class="options">
      <button v-for="(c, i) in st.choices" :key="i" @click.stop="pick(i)"><span class="ptr">▸</span> {{ i + 1 }}. {{ c }}</button>
    </div>
  </div>
</template>

<style scoped>
.jrpg {
  position: relative;
  overflow: hidden;
  color: #f4f1ff;
  cursor: pointer;
}
.scene {
  position: absolute;
  inset: 0;
  background: linear-gradient(160deg, #141a4a, #2a1d55) center / cover;
  animation: drift 45s ease-in-out infinite alternate;
}
@keyframes drift {
  from { transform: scale(1); }
  to { transform: scale(1.03); }
}
.window {
  position: absolute;
  right: 5%;
  bottom: 15%;
  left: 5%;
  display: flex;
  gap: 18px;
  padding: 16px 20px;
  border: 2px solid #c9a75a;
  border-radius: 14px;
  background: rgba(14, 20, 68, 0.88);
  box-shadow: 0 0 0 4px rgba(20, 26, 80, 0.9), 0 0 0 6px rgba(201, 167, 90, 0.6), 0 8px 30px rgba(0, 0, 0, 0.6);
}
.cinema .window {
  background: rgba(0, 0, 0, 0.45);
}
.face {
  display: grid;
  flex: none;
  place-items: center;
  width: 128px;
  height: 128px;
  overflow: hidden;
  border: 2px solid #c9a75a;
  border-radius: 8px;
  background: #1a2160;
  font-size: 48px;
  font-weight: 600;
}
.face img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.words {
  flex: 1;
  min-width: 0;
}
.name {
  display: block;
  margin-bottom: 6px;
  padding-bottom: 4px;
  border-bottom: 1px solid rgba(201, 167, 90, 0.5);
  color: #f0d79a;
  font-size: 14px;
  letter-spacing: 0.2em;
}
.words p {
  margin: 0;
  font-size: 18px;
  line-height: 1.55;
}
.words p.narration {
  font-style: italic;
  color: #dcd8f7;
}
.tri {
  font-size: 11px;
  animation: blink 1s infinite;
}
@keyframes blink {
  50% { opacity: 0.15; }
}
.say {
  margin-top: 10px;
}
.options {
  position: absolute;
  right: 5%;
  bottom: 3%;
  left: 5%;
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  justify-content: center;
}
.options button {
  padding: 8px 18px;
  border: 2px solid #c9a75a;
  border-radius: 10px;
  background: rgba(14, 20, 68, 0.9);
  color: #f4f1ff;
  font: inherit;
  font-size: 14px;
  cursor: pointer;
}
.ptr {
  color: #f0d79a;
}
</style>
