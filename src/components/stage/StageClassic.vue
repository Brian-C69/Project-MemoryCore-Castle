<script setup>
// Look A - Classic VN: the scene fills the screen, the companion and the speaker stand half-body
// over it (the speaker lit), a glass box with a name plate at the bottom, choices in the middle.
import { computed, ref } from 'vue'
import SayBox from './SayBox.vue'

const props = defineProps({ st: { type: Object, required: true }, world: { type: Object, required: true }, companion: { type: String, default: '' } })
const sayBox = ref(null)
const speaker = computed(() => props.st.current?.speaker ?? null)
// left: the companion; right: whoever else speaks (the hero by default)
const left = computed(() => ({ name: props.companion, face: props.st.faceOf(props.companion) }))
const rightName = computed(() => (speaker.value && speaker.value !== props.companion ? speaker.value : 'hero'))
const right = computed(() => ({ name: rightName.value, face: props.st.faceOf(rightName.value) }))
function pick(i) {
  if (props.st.choose(i) === 'say') sayBox.value?.focus()
}
</script>

<template>
  <div class="classic" :class="{ cinema: st.s.cinema }" @click="st.next()">
    <div class="scene" :style="st.s.scene ? { backgroundImage: `url(&quot;${st.s.scene}&quot;)` } : {}"></div>
    <div class="scene-sharp" :style="st.s.scene ? { backgroundImage: `url(&quot;${st.s.scene}&quot;)` } : {}"></div>
    <p v-if="st.s.shown?.chapter && !st.s.cinema" class="chapter">{{ st.s.shown.chapter }}</p>
    <template v-if="!st.s.clean && !st.s.cinema">
      <img v-if="left.face" :src="left.face" alt="" class="sprite left" :class="{ lit: speaker === left.name }" />
      <img v-if="right.face" :src="right.face" alt="" class="sprite right" :class="{ lit: speaker === right.name }" />
    </template>
    <div v-if="st.choices.length && !st.s.clean" class="choices">
      <button v-for="(c, i) in st.choices" :key="i" @click.stop="pick(i)">✦ {{ i + 1 }}. {{ c }}</button>
    </div>
    <div v-if="st.current && !st.s.clean" class="box">
      <span v-if="speaker" class="plate">✦ {{ speaker.toUpperCase() }}</span>
      <p class="line" :class="{ narration: !speaker }">{{ st.s.typed }}</p>
      <span v-if="!st.s.typing" class="more">▼</span>
      <SayBox v-if="st.atEnd && !st.s.cinema" ref="sayBox" class="say" @say="st.speak($event)" />
    </div>
  </div>
</template>

<style scoped>
.classic {
  position: relative;
  overflow: hidden;
  cursor: pointer;
  color: #f1efff;
}
.scene,
.scene-sharp {
  position: absolute;
  inset: 0;
  background: linear-gradient(160deg, #141a4a, #2a1d55) center / cover;
}
.scene {
  filter: blur(18px) brightness(0.6);
  transform: scale(1.1);
}
.scene-sharp {
  background-size: contain;
  background-repeat: no-repeat;
  animation: drift 40s ease-in-out infinite alternate;
}
@keyframes drift {
  from { transform: scale(1) translateX(0); }
  to { transform: scale(1.03) translateX(-1%); }
}
.chapter {
  position: absolute;
  top: 14px;
  left: 18px;
  margin: 0;
  padding: 4px 12px;
  border-radius: 8px;
  background: rgba(6, 10, 32, 0.55);
  font-size: 12px;
  letter-spacing: 0.18em;
}
.sprite {
  position: absolute;
  bottom: 0;
  height: 82%;
  max-width: 34%;
  object-fit: contain;
  object-position: bottom;
  filter: brightness(0.7) saturate(0.85);
  transition: filter 0.3s, transform 0.3s;
}
.sprite.left {
  left: 2%;
}
.sprite.right {
  right: 2%;
}
.sprite.lit {
  filter: none;
  transform: translateY(-1%);
}
.choices {
  position: absolute;
  top: 30%;
  left: 50%;
  display: flex;
  flex-direction: column;
  gap: 10px;
  transform: translateX(-50%);
}
.choices button {
  min-width: 280px;
  padding: 10px 22px;
  border: 1px solid rgba(170, 190, 255, 0.55);
  border-radius: 12px;
  background: linear-gradient(90deg, rgba(40, 60, 170, 0.75), rgba(60, 40, 140, 0.75));
  box-shadow: 0 0 18px rgba(120, 150, 255, 0.35);
  color: #f3f1ff;
  font: inherit;
  font-size: 15px;
  text-align: left;
  cursor: pointer;
}
.box {
  position: absolute;
  right: 4%;
  bottom: 4%;
  left: 4%;
  min-height: 120px;
  padding: 22px 26px 16px;
  border: 1px solid rgba(170, 190, 255, 0.45);
  border-radius: 16px;
  background: rgba(16, 22, 70, 0.72);
  backdrop-filter: blur(6px);
}
.cinema .box {
  background: none;
  border: none;
  backdrop-filter: none;
  text-align: center;
  text-shadow: 0 2px 6px #000;
}
.plate {
  position: absolute;
  top: -14px;
  left: 22px;
  padding: 3px 14px;
  border: 1px solid rgba(170, 190, 255, 0.55);
  border-radius: 8px;
  background: rgba(30, 40, 120, 0.95);
  font-size: 13px;
  letter-spacing: 0.15em;
}
.line {
  margin: 0;
  font-size: 18px;
  line-height: 1.55;
}
.line.narration {
  font-style: italic;
  color: #dad7f6;
}
.more {
  position: absolute;
  right: 18px;
  bottom: 10px;
  font-size: 12px;
  animation: blink 1.2s infinite;
}
@keyframes blink {
  50% { opacity: 0.2; }
}
.say {
  margin-top: 12px;
}
</style>
