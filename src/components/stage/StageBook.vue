<script setup>
// Look C - Storybook: an open book - the scene as the left page's picture, the story on the right
// page in a book serif (the beat's lines so far, the current one typing), choices inked below.
import { computed, ref } from 'vue'
import SayBox from './SayBox.vue'

const props = defineProps({ st: { type: Object, required: true }, world: { type: Object, required: true }, companion: { type: String, default: '' } })
const sayBox = ref(null)
// the pages already read in this beat stay on the page, as in a book
const read = computed(() => props.st.pages.slice(0, props.st.s.page))
const speaker = computed(() => props.st.current?.speaker ?? null)
const face = computed(() => props.st.faceOf(speaker.value || props.companion))
function pick(i) {
  if (props.st.choose(i) === 'say') sayBox.value?.focus()
}
</script>

<template>
  <div class="book-look" :class="{ cinema: st.s.cinema }" @click="st.next()">
    <div class="book">
      <div class="page left">
        <div class="picture" :style="st.s.scene ? { backgroundImage: `url(&quot;${st.s.scene}&quot;)` } : {}"></div>
      </div>
      <div v-if="!st.s.cinema" class="page right">
        <img v-if="face" :src="face" alt="" class="portrait" />
        <p v-if="st.s.shown?.chapter" class="chapter">{{ st.s.shown.chapter }}</p>
        <div class="text">
          <p v-for="(p, i) in read" :key="i" :class="{ speech: p.speaker }"><template v-if="p.speaker">{{ p.speaker }}: “{{ p.text }}”</template><template v-else>{{ p.text }}</template></p>
          <p v-if="st.current" :class="{ speech: speaker }"><template v-if="speaker">{{ speaker }}: “{{ st.s.typed }}”</template><template v-else>{{ st.s.typed }}</template></p>
        </div>
        <ol v-if="st.choices.length" class="choices">
          <li v-for="(c, i) in st.choices" :key="i"><button @click.stop="pick(i)">{{ c }}</button></li>
        </ol>
        <SayBox v-if="st.atEnd" ref="sayBox" class="say" placeholder="Write your action…" @say="st.speak($event)" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.book-look {
  display: grid;
  place-items: center;
  padding: 2% 3%;
  background: radial-gradient(ellipse at center, #1b2266, #060a24);
  cursor: pointer;
}
.book {
  display: grid;
  grid-template-columns: 1fr 1fr;
  width: 100%;
  height: 100%;
  max-height: 100%;
  padding: 14px;
  border-radius: 10px;
  background: #5a3b23;
  box-shadow: 0 10px 40px rgba(0, 0, 0, 0.6);
}
.cinema .book {
  grid-template-columns: 1fr;
}
.page {
  position: relative;
  min-height: 0;
  overflow: hidden;
  background: #f1e6cd;
}
.page.left {
  padding: 18px;
  border-radius: 6px 0 0 6px;
  box-shadow: inset -12px 0 18px -10px rgba(0, 0, 0, 0.35);
}
.picture {
  width: 100%;
  height: 100%;
  border: 1px solid #c9b48c;
  background: linear-gradient(160deg, #2a3170, #3b2a66) center / cover;
}
.page.right {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 28px 30px 20px;
  border-radius: 0 6px 6px 0;
  box-shadow: inset 12px 0 18px -10px rgba(0, 0, 0, 0.3);
  color: #3a2a18;
  font-family: Georgia, 'Times New Roman', serif;
}
.portrait {
  position: absolute;
  top: 16px;
  right: 18px;
  width: 64px;
  height: 64px;
  border: 2px solid #b79a64;
  border-radius: 50%;
  object-fit: cover;
}
.chapter {
  margin: 0;
  font-size: 13px;
  letter-spacing: 0.2em;
  text-transform: uppercase;
  color: #7a5a30;
}
.text {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding-right: 70px;
}
.text p {
  margin: 0 0 10px;
  font-size: 16px;
  line-height: 1.6;
}
.text p.speech {
  font-style: italic;
}
.choices {
  margin: 0;
  padding-left: 22px;
}
.choices button {
  padding: 2px 0;
  border: none;
  background: none;
  color: #4a2f12;
  font: inherit;
  font-size: 15px;
  text-align: left;
  cursor: pointer;
}
.choices button:hover {
  text-decoration: underline;
}
.say :deep(input) {
  border-color: #c9b48c;
  background: rgba(255, 255, 255, 0.4);
  color: #3a2a18;
  font-family: Georgia, serif;
}
.say :deep(button) {
  border-color: #c9b48c;
  background: #e2d2ae;
  color: #3a2a18;
}
</style>
