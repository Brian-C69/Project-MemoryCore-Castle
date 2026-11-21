<script setup>
// One reader for three rooms: DIARY (by month, newest first), PROJECTS (active, paused, archived,
// recently deleted - the open project can be moved between them; each parent opens and closes
// on its sub-projects, and every project keeps a task list the member edits here) and
// LIBRARY (by section). The list is on the left, the page on the right. The words are read-only:
// only the companion writes them (a project's shelf is the one thing the member moves here). The DIARY can also be read as a calendar or as a book - the
// same pages, the member's choice, remembered.
import { computed, onMounted, ref, watch } from 'vue'
import MarkdownText from '../components/MarkdownText.vue'
import ProjectShelfBar from '../components/ProjectShelfBar.vue'
import ProjectTasks from '../components/ProjectTasks.vue'
import ProjectSplit from '../components/ProjectSplit.vue'
import { treeRows } from '../components/projectshelf.js'
import DiaryBook from '../components/DiaryBook.vue'
import DiaryCalendar from '../components/DiaryCalendar.vue'
import DiaryPage from '../components/DiaryPage.vue'
import { bookPages, lookOf, PAGE_DESIGNS } from '../components/diaryviews.js'
import { chatMediaOpen, getSettings, memoryRead, memoryShelves } from '../api/castle.js'
import { absPath } from './dev/tree.js'

const props = defineProps({
  room: { type: String, required: true }, // diary | projects | library
  // changes when the memory may have changed (HOME's minute refresh, Settings)
  stamp: { type: Number, default: 0 },
  // a file to open first (from a HOME card)
  openFile: { type: String, default: '' },
})

const EMPTY = {
  diary: 'No diary pages yet. Press Save diary after a talk with your companion.',
  projects: 'No projects yet. Ask your companion to start one.',
  library: 'The library is empty. Ask your companion to save something worth keeping.',
}
const TITLES = { diary: 'DIARY', projects: 'PROJECTS', library: 'LIBRARY' }

const shelves = ref([])
const query = ref('')
const current = ref(null)
const text = ref('')
const problem = ref('')
const memoryFolder = ref('')
// how the DIARY is read: list | calendar | book (the other rooms are always a list)
const diaryLook = ref('list')
const look = computed(() => (props.room === 'diary' ? diaryLook.value : 'list'))
// the companion's name signs a diary page; every diary page oldest first, for the days around one
const companion = ref('')
const diaryPages = computed(() => (props.room === 'diary' ? bookPages(shelves.value) : []))

// The diary shows the one look chosen in the THEME room or Settings > Rooms (no switcher here).

// PROJECTS: the parents the member closed, and a stamp that makes the open project read again
// after a task or a move changes it
const closed = ref(new Set())
const projectStamp = ref(0)
const tree = computed(() => (props.room === 'projects' ? treeRows(shelves.value, query.value, closed.value) : []))

function toggle(page) {
  const next = new Set(closed.value)
  if (next.has(page.file)) next.delete(page.file)
  else next.add(page.file)
  closed.value = next
}

// a parent row: the first click opens the page, a click on the open one closes or opens it
function pick(row) {
  if (row.kids && current.value?.file === row.page.file) toggle(row.page)
  else {
    if (row.kids && closed.value.has(row.page.file)) toggle(row.page)
    open(row.page)
  }
}

function openFile(file) {
  const page = shelves.value.flatMap((s) => s.pages).find((p) => p.file === file)
  if (page) open(page)
}

function changed() {
  projectStamp.value++
  load()
}

const filtered = computed(() => {
  const q = query.value.trim().toLowerCase()
  if (!q) return shelves.value
  return shelves.value
    .map((s) => ({ ...s, pages: s.pages.filter((p) => p.title.toLowerCase().includes(q) || (p.date ?? '').includes(q)) }))
    .filter((s) => s.pages.length)
})

function monthName(name) {
  if (!/^\d{4}-\d{2}$/.test(name)) return name
  const d = new Date(`${name}-01T00:00:00`)
  return d.toLocaleDateString('en-US', { month: 'long', year: 'numeric' })
}

function dayName(date) {
  if (!date) return ''
  const d = new Date(`${date}T00:00:00`)
  return Number.isNaN(d.getTime()) ? date : d.toLocaleDateString('en-US', { weekday: 'short', day: 'numeric' })
}

// `refresh` reads the open page again in place: its text stays until the new text arrives and is
// replaced only when it differs, so the reader keeps their place.
async function open(page, refresh = false) {
  current.value = page
  if (!refresh) {
    problem.value = ''
    text.value = ''
  }
  try {
    const got = await memoryRead(page.file)
    if (got !== text.value) text.value = got
    problem.value = ''
  } catch (reason) {
    problem.value = String(reason)
  }
}

function openOutside() {
  if (!current.value || !memoryFolder.value) return
  chatMediaOpen(absPath(memoryFolder.value, current.value.file), true).catch(() => {})
}

async function load() {
  try {
    // HOME refreshes every minute: the same shelves keep their identity, so nothing redraws
    const fresh = await memoryShelves(props.room)
    if (JSON.stringify(fresh) !== JSON.stringify(shelves.value)) shelves.value = fresh
    const settings = (await getSettings()).settings
    memoryFolder.value = settings.memoryFolder ?? ''
    diaryLook.value = lookOf(settings.diaryLook)
    companion.value = settings.companionName ?? ''
  } catch {
    shelves.value = []
  }
  const wanted = props.openFile && shelves.value.flatMap((s) => s.pages).find((p) => p.file === props.openFile)
  const keep = current.value && shelves.value.flatMap((s) => s.pages).find((p) => p.file === current.value.file)
  const first = shelves.value[0]?.pages[0]
  const next = wanted || keep || first
  if (next && next.file !== current.value?.file) open(next)
  else if (keep) open(keep, true)
}

onMounted(load)
watch(() => [props.room, props.stamp, props.openFile], load)
</script>

<template>
  <!-- the DIARY as a book: two pages across the whole room -->
  <div v-if="look === 'book'" class="memory-room book-room">
    <div class="looks top">
      <h2>{{ TITLES[room] }}</h2>
    </div>
    <DiaryBook :shelves="shelves" :open-file="openFile" :companion="companion" @open="open" />
  </div>
  <div v-else class="memory-room">
    <aside class="list widget">
      <h2>{{ TITLES[room] }}</h2>
      <DiaryCalendar v-if="look === 'calendar'" :shelves="shelves" :current="current" @open="open" />
      <template v-else>
      <input v-model="query" type="search" placeholder="Search" aria-label="Search" />
      <p v-if="!shelves.length" class="empty">{{ EMPTY[room] }}</p>
      <ProjectSplit v-if="room === 'projects'" :stamp="stamp" @applied="changed" />
      <template v-if="room === 'projects'">
        <div v-for="shelf in tree" :key="shelf.name" class="shelf">
          <h3>{{ shelf.name }}</h3>
          <div v-for="row in shelf.rows" :key="row.page.file" class="tree-row" :class="{ kid: row.depth, elsewhere: row.page.tree?.elsewhere }">
            <button
              v-if="row.kids"
              class="twisty"
              :aria-expanded="row.open"
              :aria-label="`${row.open ? 'Close' : 'Open'} the sub-projects of ${row.page.title}`"
              @click="toggle(row.page)"
            >▸</button>
            <button :class="{ active: current?.file === row.page.file, parent: row.kids }" :title="row.page.tree?.elsewhere ? `${row.page.title} itself is ${row.page.tree.elsewhere}` : row.page.title" @click="pick(row)">
              <span class="title">{{ row.page.title }}</span>
              <span v-if="row.page.tree?.tasks || row.kids" class="count">
                <template v-if="row.kids">{{ row.kids }}<template v-if="row.page.tree?.tasks"> · </template></template>
                <template v-if="row.page.tree?.tasks">{{ row.page.tree.done }}/{{ row.page.tree.tasks }}</template>
              </span>
            </button>
          </div>
        </div>
      </template>
      <div v-for="shelf in room === 'projects' ? [] : filtered" :key="shelf.name" class="shelf">
        <h3>{{ room === 'diary' ? monthName(shelf.name) : shelf.name }}</h3>
        <button
          v-for="page in shelf.pages"
          :key="page.file"
          :class="{ active: current?.file === page.file }"
          @click="open(page)"
        >
          <span v-if="page.date" class="date">{{ dayName(page.date) }}</span>
          <span class="title">{{ page.title }}</span>
        </button>
      </div>
      </template>
    </aside>
    <article class="page widget">
      <!-- a diary entry wears the page that goes with the way it is browsed -->
      <template v-if="current && room === 'diary'">
        <button v-if="!current.file.startsWith('db/')" class="small show-file" title="Show the file in its folder" @click="openOutside">Show file</button>
        <DiaryPage :design="PAGE_DESIGNS[look]" :page="current" :text="text" :problem="problem" :pages="diaryPages" :companion="companion" @open="open" />
      </template>
      <template v-else-if="current">
        <header>
          <h1>{{ current.title }}</h1>
          <!-- a page of the database has no file to show -->
          <button v-if="!current.file.startsWith('db/')" class="small" title="Show the file in its folder" @click="openOutside">Show file</button>
        </header>
        <template v-if="room === 'projects' && current.file.startsWith('db/project/')">
          <ProjectShelfBar :file="current.file" :companion="companion" :stamp="projectStamp" @moved="changed" />
        </template>
        <p v-if="problem" class="problem">{{ problem }}</p>
        <div class="text">
          <ProjectTasks v-if="room === 'projects' && current.file.startsWith('db/project/')" :file="current.file" :stamp="projectStamp" @open="openFile" @changed="changed" />
          <MarkdownText :text="text" />
        </div>
      </template>
      <p v-else class="empty">{{ EMPTY[room] }}</p>
    </article>
  </div>
</template>

<style scoped>
.memory-room {
  position: absolute;
  inset: 24px;
  display: grid;
  grid-template-columns: minmax(260px, 30%) 1fr;
  gap: 18px;
}
.show-file {
  align-self: flex-end;
  margin-bottom: 6px;
}
.book-room {
  grid-template-columns: 1fr;
  grid-template-rows: auto 1fr;
  gap: 12px;
}
.looks {
  display: flex;
  gap: 6px;
  align-items: center;
}
.looks.top h2 {
  margin: 0 12px 0 4px;
}
.looks button {
  padding: 5px 12px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--panel-background);
  color: var(--text-dim);
  font: inherit;
  font-size: 13px;
  cursor: pointer;
}
.looks button.on {
  border-color: var(--accent);
  color: var(--text);
  background: var(--hover);
}
.widget {
  background: var(--panel-background);
  border: 1px solid var(--line);
  border-radius: 14px;
  -webkit-backdrop-filter: blur(var(--glass-blur)); backdrop-filter: blur(var(--glass-blur));
}
.list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 18px;
  overflow-y: auto;
}
h2 {
  margin: 0 0 4px;
  font-size: 13px;
  font-weight: 500;
  letter-spacing: 0.2em;
  color: var(--accent);
}
h3 {
  margin: 12px 0 4px;
  font-size: 12px;
  font-weight: 500;
  letter-spacing: 0.08em;
  color: var(--text-dim);
  text-transform: capitalize;
}
input {
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.25);
  color: var(--text);
  font: inherit;
  font-size: 14px;
}
.shelf button {
  display: flex;
  gap: 10px;
  width: 100%;
  padding: 7px 10px;
  border: 1px solid transparent;
  border-radius: 8px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 14px;
  text-align: left;
  cursor: pointer;
}
.shelf button:hover {
  background: var(--hover);
}
.shelf button.active {
  border-color: var(--accent);
  background: var(--hover);
}
.tree-row {
  display: flex;
  align-items: center;
  gap: 2px;
}
.tree-row > button:last-child {
  flex: 1;
  min-width: 0;
}
.tree-row.kid {
  padding-left: 22px;
}
.tree-row:not(.kid):not(:has(.twisty)) {
  padding-left: 22px;
}
.tree-row.elsewhere .title {
  color: var(--text-dim);
  font-style: italic;
}
.shelf .twisty {
  flex: none;
  width: 20px;
  padding: 7px 0;
  justify-content: center;
  color: var(--text-dim);
  font-size: 11px;
}
.twisty > * {
  pointer-events: none;
}
.twisty[aria-expanded='true'] {
  transform: rotate(90deg);
}
.shelf button.parent .title {
  font-weight: 600;
}
.count {
  flex: none;
  margin-left: auto;
  font-size: 11px;
  color: var(--text-dim);
  font-variant-numeric: tabular-nums;
}
.date {
  flex: none;
  width: 58px;
  font-size: 12px;
  color: var(--text-dim);
}
.title {
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.page {
  display: flex;
  flex-direction: column;
  min-height: 0;
  padding: 22px 28px;
}
.page header {
  display: flex;
  gap: 12px;
  align-items: center;
  margin-bottom: 12px;
}
h1 {
  flex: 1;
  margin: 0;
  font-size: 22px;
  font-weight: 500;
}
.text {
  overflow-y: auto;
  padding-right: 8px;
}
.small {
  padding: 5px 12px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 13px;
  cursor: pointer;
}
.empty {
  font-size: 14px;
  color: var(--text-dim);
}
.problem {
  font-size: 13px;
  color: var(--warn);
}
</style>
