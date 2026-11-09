<script setup>
// The STORY stage: the shared engine (useStage) and bar, and the look the member chose. Every
// look draws the same beat; only the layout differs (his rule: isolate layout, share logic).
import { computed, reactive } from 'vue'
import { useStage } from './useStage.js'
import { LOOKS } from './stage.js'
import StageClassic from './StageClassic.vue'
import StageCinema from './StageCinema.vue'
import StageBook from './StageBook.vue'
import StageMessenger from './StageMessenger.vue'
import StageJrpg from './StageJrpg.vue'

const props = defineProps({
  // a world from story_worlds
  world: { type: Object, required: true },
  companion: { type: String, default: 'Your companion' },
  // the companion's standard portrait (absolute path), its face on the stage
  portrait: { type: String, default: null },
  look: { type: String, default: 'classic' },
  active: { type: Boolean, default: true },
  // words typed into the story session
  say: { type: Function, required: true },
})
const emit = defineEmits(['shelf', 'save', 'story', 'scenes', 'cast', 'log', 'look'])

const VIEWS = { classic: StageClassic, cinema: StageCinema, book: StageBook, messenger: StageMessenger, jrpg: StageJrpg }

// reactive() unwraps the engine's computed values for the looks' templates
const st = reactive(
  useStage({
    world: () => props.world,
    companion: () => props.companion,
    portrait: () => props.portrait,
    active: () => props.active,
    say: (text) => props.say(text),
  }),
)
const view = computed(() => VIEWS[props.look] ?? StageClassic)
const hasCinema = computed(() => ['classic', 'book', 'jrpg'].includes(props.look))
</script>

<template>
  <div class="stage-view">
    <component :is="view" :st="st" :world="world" :companion="companion" class="look" />
    <div v-if="st.s.preparing && !st.s.shown" class="preparing">The story is being prepared…</div>
    <div v-else-if="!st.s.shown" class="preparing">Waiting for the story's first lines…</div>
    <nav class="bar">
      <button @click="emit('shelf')">Shelf</button>
      <button @click="emit('save')">Save</button>
      <button @click="emit('story')">Story</button>
      <button @click="emit('scenes')">Scenes</button>
      <button @click="emit('cast')">Cast</button>
      <button @click="emit('log')">Log</button>
      <span class="spacer"></span>
      <span v-if="st.s.waiting" class="ready">The next scene is ready - click to continue</span>
      <label class="look-pick">
        Look
        <select :value="look" @change="emit('look', $event.target.value)">
          <option v-for="l in LOOKS" :key="l.id" :value="l.id">{{ l.title }}</option>
        </select>
      </label>
      <button v-if="hasCinema" :class="{ on: st.s.cinema }" title="Only the scene and the words" @click="st.s.cinema = !st.s.cinema">Cinema</button>
      <button :class="{ on: st.s.music }" :title="st.s.music ? 'Music on' : 'Music off'" @click="st.s.music = !st.s.music">♪</button>
      <input v-model.number="st.s.volume" type="range" min="0" max="1" step="0.05" class="volume" aria-label="Music volume" />
    </nav>
  </div>
</template>

<style scoped>
.stage-view {
  position: relative;
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  overflow: hidden;
  border-radius: 14px;
  background: #050a1c;
}
.look {
  flex: 1;
  min-height: 0;
}
.preparing {
  position: absolute;
  top: 40%;
  left: 50%;
  padding: 8px 16px;
  border-radius: 10px;
  background: rgba(5, 10, 28, 0.7);
  color: #d9d6f5;
  font-size: 14px;
  transform: translateX(-50%);
  pointer-events: none;
}
.bar {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
  padding: 6px 10px;
  border-top: 1px solid rgba(160, 170, 255, 0.18);
  background: rgba(8, 12, 34, 0.85);
}
.bar button,
.bar select {
  padding: 4px 10px;
  border: 1px solid rgba(160, 170, 255, 0.25);
  border-radius: 8px;
  background: none;
  color: #e3e1fb;
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}
.bar select {
  background: rgba(0, 0, 0, 0.3);
}
.bar button.on {
  border-color: #9fc4ff;
  color: #9fc4ff;
}
.look-pick {
  display: flex;
  gap: 6px;
  align-items: center;
  color: #b9b6e0;
  font-size: 12px;
}
.ready {
  color: #9fc4ff;
  font-size: 12px;
}
.spacer {
  flex: 1;
}
.volume {
  width: 80px;
}
</style>
