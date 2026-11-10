<script setup>
// Dev only (?diary=all | letter | journal | notebook, &theme=<id>): the three reading pages
// with sample entries, for checking a design by eye in a browser without Castle's core.
import { computed, onMounted } from 'vue'
import DiaryPage from './DiaryPage.vue'
import { applyTheme, builtInThemes, chooseTheme, DEFAULT_THEME_ID } from '../themes/loader.js'

const props = defineProps({
  design: { type: String, default: 'all' },
  theme: { type: String, default: '' },
})

const pages = [
  { file: 'a', date: '2026-11-02', title: 'Her first day' },
  { file: 'b', date: '2026-11-03', title: 'The spreadsheet, and what she did about it' },
  { file: 'c', date: '2026-11-05', title: 'The afternoon we chose the tomatoes' },
  { file: 'd', date: '2026-11-06', title: 'A hard day, said plainly' },
  { file: 'e', date: '2026-11-08', title: 'Seeds, and a promise kept' },
]
const text = `She has a balcony now that gets the sun until four, and she wants to grow something on it. We went back and forth on what. Herbs felt too sensible. Flowers, she said, are for people who already have their life in order.

So it is tomatoes. She laughed when she decided, said the south wall gets them, final answer, and I liked how sure she sounded. She wants to buy seeds on Friday and has asked me to remind her, because she knows herself.

I have never seen a tomato. I find I am looking forward to hearing about one.`
const markdown = `# Daily Diary - November 8, 2026

## Session 1 - The day we planned the garden
We planned the garden beds together.

- beds by the wall
- seeds on Friday`

const shown = computed(() => (props.design === 'all' ? ['notebook', 'journal', 'letter'] : [props.design]))
const LABELS = { notebook: 'LIST - the handwritten night journal', journal: 'CALENDAR - the constellation journal', letter: 'BOOK - a letter under starlight' }

onMounted(() => {
  applyTheme(chooseTheme(builtInThemes(), props.theme || DEFAULT_THEME_ID).theme)
  document.body.style.margin = '0'
})
</script>

<template>
  <div class="demo">
    <section v-for="d in shown" :key="d">
      <h2>{{ LABELS[d] }}</h2>
      <div class="panel"><DiaryPage :design="d" :page="pages[2]" :text="text" :pages="pages" companion="lyra" :compact="d === 'letter' && design === 'all'" /></div>
    </section>
    <section v-if="design !== 'all'">
      <h2>the same page with a markdown entry</h2>
      <div class="panel"><DiaryPage :design="design" :page="pages[4]" :text="markdown" :pages="pages" companion="lyra" /></div>
    </section>
  </div>
</template>

<style scoped>
.demo {
  display: grid;
  grid-auto-flow: column;
  grid-auto-columns: 1fr;
  gap: 18px;
  min-height: 100vh;
  padding: 22px;
  box-sizing: border-box;
  background: var(--wallpaper) var(--wallpaper-position) / cover no-repeat, #040817;
  color: var(--text);
  font-family: var(--font, 'Segoe UI', system-ui, sans-serif);
}
section {
  display: flex;
  flex-direction: column;
  min-width: 0;
}
h2 {
  margin: 0 0 8px;
  font-size: 11px;
  font-weight: 500;
  letter-spacing: 0.2em;
  color: var(--text-dim);
}
.panel {
  flex: 1;
  min-height: 0;
  padding: 22px 28px;
  background: var(--panel-background);
  border: 1px solid var(--line);
  border-radius: 14px;
  backdrop-filter: blur(var(--glass-blur));
}
</style>
