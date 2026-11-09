<script setup>
// Look B - Cinematic: the wide scene letterboxed like a film, the words as subtitles in the lower
// bar, the choices as a slim list at the right, small round cameos at the left.
import { computed, ref } from 'vue'
import SayBox from './SayBox.vue'

const props = defineProps({ st: { type: Object, required: true }, world: { type: Object, required: true }, companion: { type: String, default: '' } })
const sayBox = ref(null)
const speaker = computed(() => props.st.current?.speaker ?? null)
const cameos = computed(() => {
  const names = [props.companion, ...new Set((props.st.s.shown?.lines ?? []).map((l) => l.speaker).filter((n) => n && n !== props.companion))].slice(0, 3)
  return names.map((name) => ({ name, face: props.st.faceOf(name), letter: props.st.initial(name) }))
})
function pick(i) {
  if (props.st.choose(i) === 'say') sayBox.value?.focus()
}
</script>

<template>
  <div class="cinema-look" @click="st.next()">
    <div class="bars top"></div>
    <div class="frame">
      <div class="scene" :style="st.s.scene ? { backgroundImage: `url(&quot;${st.s.scene}&quot;)` } : {}"></div>
      <div v-if="st.choices.length && !st.s.clean" class="choices">
        <button v-for="(c, i) in st.choices" :key="i" @click.stop="pick(i)">✦ {{ i + 1 }}. {{ c }}</button>
      </div>
    </div>
    <div class="bars bottom">
      <div class="cameos">
        <span v-for="c in cameos" :key="c.name" class="cameo" :class="{ lit: c.name === speaker }" :title="c.name">
          <img v-if="c.face" :src="c.face" alt="" /><template v-else>{{ c.letter }}</template>
        </span>
      </div>
      <div v-if="st.current && !st.s.clean" class="subtitle">
        <span v-if="speaker" class="who">— {{ speaker.toUpperCase() }} —</span>
        <p :class="{ narration: !speaker }">{{ st.s.typed }}</p>
        <SayBox v-if="st.atEnd" ref="sayBox" class="say" @say="st.speak($event)" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.cinema-look {
  display: flex;
  flex-direction: column;
  background: #000;
  color: #f1efff;
  cursor: pointer;
}
.bars.top {
  height: 7%;
}
.frame {
  position: relative;
  flex: 1;
  min-height: 0;
}
.scene {
  position: absolute;
  inset: 0;
  background: linear-gradient(160deg, #141a4a, #2a1d55) center / cover;
  animation: drift 50s ease-in-out infinite alternate;
}
@keyframes drift {
  from { transform: scale(1); }
  to { transform: scale(1.04); }
}
.choices {
  position: absolute;
  top: 6%;
  right: 2%;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.choices button {
  min-width: 210px;
  padding: 7px 14px;
  border: 1px solid rgba(200, 210, 255, 0.35);
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.55);
  color: #eceaff;
  font: inherit;
  font-size: 13px;
  text-align: left;
  cursor: pointer;
}
.bars.bottom {
  position: relative;
  display: flex;
  gap: 16px;
  align-items: center;
  min-height: 22%;
  padding: 10px 20px;
}
.cameos {
  display: flex;
  gap: 8px;
}
.cameo {
  display: grid;
  place-items: center;
  width: 44px;
  height: 44px;
  overflow: hidden;
  border: 1px solid rgba(200, 210, 255, 0.4);
  border-radius: 50%;
  background: #12163a;
  opacity: 0.55;
  font-weight: 600;
}
.cameo.lit {
  opacity: 1;
  box-shadow: 0 0 10px rgba(160, 180, 255, 0.6);
}
.cameo img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.subtitle {
  flex: 1;
  text-align: center;
}
.who {
  font-size: 11px;
  letter-spacing: 0.3em;
  color: #b9b6e0;
}
.subtitle p {
  margin: 6px 0 0;
  font-size: 17px;
  line-height: 1.5;
}
.subtitle p.narration {
  font-style: italic;
  color: #cfcbef;
}
.say {
  max-width: 520px;
  margin: 10px auto 0;
}
</style>
