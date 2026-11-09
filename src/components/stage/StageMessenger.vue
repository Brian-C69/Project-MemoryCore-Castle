<script setup>
// Look D - Messenger: the adventure as a chat thread with the cast - narration cards, a new scene
// as a picture in the thread, speakers in bubbles with their faces, choices as quick replies.
import { computed, nextTick, ref, watch } from 'vue'
import SayBox from './SayBox.vue'

const props = defineProps({ st: { type: Object, required: true }, world: { type: Object, required: true }, companion: { type: String, default: '' } })
const sayBox = ref(null)
const thread = ref(null)

// every beat seen this visit, its lines all shown (the newest beat up to the page being typed)
const items = computed(() => {
  const out = []
  let lastScene = null
  const history = props.st.s.history ?? []
  history.forEach((beat, b) => {
    if (beat.sceneUrl && beat.sceneUrl !== lastScene) {
      out.push({ kind: 'scene', src: beat.sceneUrl, key: `s${b}` })
      lastScene = beat.sceneUrl
    }
    const newest = b === history.length - 1
    const lines = newest ? props.st.pages.slice(0, props.st.s.page + 1) : beat.lines
    lines.forEach((line, i) => {
      const typing = newest && i === props.st.s.page
      out.push({ kind: line.speaker ? 'speech' : 'narration', speaker: line.speaker, text: typing ? props.st.s.typed : line.text, key: `${b}-${i}` })
    })
  })
  return out
})
watch(() => [items.value.length, props.st.s.typed], () => nextTick(() => thread.value && (thread.value.scrollTop = thread.value.scrollHeight)))
function pick(i) {
  if (props.st.choose(i) === 'say') sayBox.value?.focus()
}
</script>

<template>
  <div class="messenger" @click="st.next()">
    <header>
      <strong>{{ world.name }}</strong>
      <span v-if="st.s.shown?.chapter">· {{ st.s.shown.chapter }}</span>
    </header>
    <div ref="thread" class="thread">
      <template v-for="it in items" :key="it.key">
        <img v-if="it.kind === 'scene'" :src="it.src" alt="" class="scene" />
        <p v-else-if="it.kind === 'narration'" class="narration">{{ it.text }}</p>
        <div v-else class="speech">
          <span class="face"><img v-if="st.faceOf(it.speaker)" :src="st.faceOf(it.speaker)" alt="" /><template v-else>{{ st.initial(it.speaker) }}</template></span>
          <div class="bubble">
            <span class="who">{{ it.speaker }}</span>
            <span>{{ it.text }}</span>
          </div>
        </div>
      </template>
    </div>
    <div class="replies" @click.stop>
      <button v-for="(c, i) in st.choices" :key="i" @click="pick(i)">✦ {{ c }}</button>
    </div>
    <SayBox ref="sayBox" class="say" placeholder="Type what you do…" @say="st.speak($event)" />
  </div>
</template>

<style scoped>
.messenger {
  display: flex;
  flex-direction: column;
  min-height: 0;
  padding: 12px 16px;
  background: linear-gradient(180deg, #0c1238, #090d2a);
  color: #eceaff;
  cursor: pointer;
}
header {
  display: flex;
  gap: 8px;
  padding-bottom: 8px;
  border-bottom: 1px solid rgba(160, 170, 255, 0.2);
  font-size: 13px;
  color: #c4c1ea;
}
header strong {
  color: #eceaff;
}
.thread {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 10px;
  min-height: 0;
  overflow-y: auto;
  padding: 12px 4px;
}
.scene {
  align-self: center;
  max-width: 70%;
  max-height: 260px;
  border: 1px solid rgba(160, 170, 255, 0.3);
  border-radius: 12px;
  object-fit: cover;
}
.narration {
  align-self: center;
  max-width: 80%;
  margin: 0;
  padding: 8px 14px;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.06);
  color: #d4d1f3;
  font-size: 13px;
  font-style: italic;
  text-align: center;
}
.speech {
  display: flex;
  gap: 8px;
  align-items: flex-end;
  max-width: 75%;
}
.face {
  display: grid;
  flex: none;
  place-items: center;
  width: 34px;
  height: 34px;
  overflow: hidden;
  border-radius: 50%;
  background: rgba(140, 150, 255, 0.25);
  font-weight: 600;
}
.face img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.bubble {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 8px 12px;
  border-radius: 14px 14px 14px 4px;
  background: rgba(70, 90, 210, 0.35);
  font-size: 14px;
}
.who {
  font-size: 11px;
  color: #aab4ff;
}
.replies {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  padding: 8px 0;
}
.replies button {
  padding: 6px 14px;
  border: 1px solid rgba(170, 180, 255, 0.45);
  border-radius: 999px;
  background: rgba(60, 80, 200, 0.3);
  color: #eceaff;
  font: inherit;
  font-size: 13px;
  cursor: pointer;
}
</style>
