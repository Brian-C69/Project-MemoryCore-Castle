<script setup>
// The DIARY room as a book: two pages side by side, oldest to newest, opening at the newest.
// The arrows (and the left and right keys) turn a spread at a time.
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import MarkdownText from './MarkdownText.vue'
import { memoryRead } from '../api/castle.js'
import { bookPages, longDate, openSpread, spreadAt, spreadLabel, turn } from './diaryviews.js'

const props = defineProps({
  shelves: { type: Array, default: () => [] },
  // a page to open at (from a HOME card)
  openFile: { type: String, default: '' },
})
const emit = defineEmits(['open'])

const pages = computed(() => bookPages(props.shelves))
const start = ref(0)
const spread = computed(() => spreadAt(pages.value, start.value))
const texts = ref({}) // file -> text | { problem }

async function read(page) {
  if (!page || texts.value[page.file] !== undefined) return
  try {
    texts.value = { ...texts.value, [page.file]: await memoryRead(page.file) }
  } catch (reason) {
    texts.value = { ...texts.value, [page.file]: { problem: String(reason) } }
  }
}

function go(by) {
  start.value = turn(start.value, by, pages.value.length)
}

function onKey(event) {
  if (event.target?.tagName === 'INPUT' || event.target?.tagName === 'TEXTAREA') return
  if (event.key === 'ArrowLeft') go(-1)
  if (event.key === 'ArrowRight') go(1)
}

watch(
  [pages, () => props.openFile],
  ([list, file], [, oldFile] = []) => {
    // the first pages, or a new page asked for: open there; a refresh keeps the place
    if (file !== oldFile || start.value >= list.length) start.value = openSpread(list, file)
  },
  { immediate: true },
)
watch(
  spread,
  (now) => {
    read(now.left)
    read(now.right)
    if (now.left) emit('open', now.left)
  },
  { immediate: true },
)
// the memory may have changed: read the shown pages again
watch(
  () => props.shelves,
  () => {
    texts.value = {}
    read(spread.value.left)
    read(spread.value.right)
  },
)

onMounted(() => window.addEventListener('keydown', onKey))
onBeforeUnmount(() => window.removeEventListener('keydown', onKey))
</script>

<template>
  <div class="book">
    <div class="spread">
      <article v-for="(page, side) in [spread.left, spread.right]" :key="side" class="leaf" :class="side ? 'right' : 'left'">
        <template v-if="page">
          <p class="date">{{ longDate(page.date) || 'Earlier' }}</p>
          <h2>{{ page.title }}</h2>
          <div class="text">
            <p v-if="texts[page.file]?.problem" class="problem">{{ texts[page.file].problem }}</p>
            <MarkdownText v-else :text="typeof texts[page.file] === 'string' ? texts[page.file] : ''" />
          </div>
        </template>
        <p v-else class="blank">{{ pages.length ? 'The next page is not written yet.' : 'No diary pages yet. Press Save diary after a talk with your companion.' }}</p>
      </article>
    </div>
    <footer>
      <button :disabled="start === 0" @click="go(-1)">‹ Older</button>
      <span>{{ spreadLabel(start, pages.length) }}</span>
      <button :disabled="turn(start, 1, pages.length) === start" @click="go(1)">Newer ›</button>
    </footer>
  </div>
</template>

<style scoped>
.book {
  display: flex;
  flex-direction: column;
  min-height: 0;
  height: 100%;
  gap: 12px;
}
.spread {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: 1fr 1fr;
  background: var(--panel-background);
  border: 1px solid var(--line);
  border-radius: 14px;
  backdrop-filter: blur(var(--glass-blur));
}
.leaf {
  display: flex;
  flex-direction: column;
  min-height: 0;
  padding: 26px 30px;
}
.leaf.left {
  border-right: 1px solid var(--line);
}
.date {
  margin: 0 0 4px;
  font-size: 12px;
  letter-spacing: 0.08em;
  color: var(--accent);
}
h2 {
  margin: 0 0 14px;
  font-size: 21px;
  font-weight: 500;
}
.text {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding-right: 8px;
  line-height: 1.65;
}
.blank,
.problem {
  margin: auto;
  font-size: 14px;
  color: var(--text-dim);
  text-align: center;
}
.problem {
  color: var(--warn);
}
footer {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 22px;
  font-size: 13px;
  color: var(--text-dim);
}
footer button {
  padding: 6px 14px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--panel-background);
  color: var(--text);
  font: inherit;
  font-size: 13px;
  cursor: pointer;
}
footer button:disabled {
  opacity: 0.35;
  cursor: default;
}
</style>
