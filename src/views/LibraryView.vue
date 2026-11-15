<script setup>
// LIBRARY: the member's own books (rows in their memory database; the companion's "save library"
// keeps them) and the Spiral Library (Kiyo's published books, pulled from the Portfolio by Castle
// alone, read-only; a book read whole can be kept in My books). Shelves on the left, book cards in
// the middle, the reader on the right.
import { computed, onMounted, ref, watch } from 'vue'
import MarkdownText from '../components/MarkdownText.vue'
import { libraryBooks, libraryDelete, libraryRead, memberSignIn, spiralKeep, spiralKept, spiralRead, spiralRoom } from '../api/castle.js'
import { filterBooks, keepState, previewLine, priceOf, shelvesOf } from '../components/library.js'

const props = defineProps({
  companion: { type: String, default: 'Your companion' },
  stamp: { type: Number, default: 0 },
})

const SIDE_KEY = 'castle.library.side'
function rememberedSide() {
  try {
    return localStorage.getItem(SIDE_KEY) === 'spiral' ? 'spiral' : 'mine'
  } catch {
    return 'mine'
  }
}
const side = ref(rememberedSide())
watch(side, (s) => {
  try {
    localStorage.setItem(SIDE_KEY, s)
  } catch {
    // the choice still holds until Castle closes
  }
  shelf.value = ''
  open.value = null
})

const mine = ref([])
const spiral = ref({ shelves: [], books: [], signedIn: false })
const problem = ref('')
const loading = ref(false)
const shelf = ref('')
const query = ref('')
const open = ref(null) // { title, body, preview, book }
const reading = ref(false)
const asking = ref(null)
const kept = ref({}) // Spiral ref -> the edition in My books
const keeping = ref(false)
const KEEP_WORDS = { keep: 'Add to My books', kept: 'In My books', update: 'Update in My books' }

const books = computed(() => (side.value === 'mine' ? mine.value : spiral.value.books))
const shelfLabels = computed(() => Object.fromEntries(spiral.value.shelves.map((s) => [s.key, s.label])))
const shelves = computed(() => shelvesOf(books.value, side.value === 'spiral' ? shelfLabels.value : {}))
const shown = computed(() => filterBooks(books.value, shelf.value, query.value))
const preview = computed(() => (open.value?.spiral ? previewLine(open.value.spiral, spiral.value.signedIn) : ''))
const keep = computed(() => (open.value?.spiral ? keepState(open.value.spiral, kept.value) : 'none'))

// which Spiral books are already in My books; without the memory database nothing is, and the
// keep itself says why
async function loadKept() {
  try {
    kept.value = await spiralKept()
  } catch {
    kept.value = {}
  }
}

async function keepBook() {
  const book = open.value?.spiral
  if (!book) return
  problem.value = ''
  keeping.value = true
  try {
    mine.value = await spiralKeep(book.ref)
    await loadKept()
  } catch (e) {
    problem.value = String(e?.message ?? e)
  } finally {
    keeping.value = false
  }
}

async function load() {
  problem.value = ''
  loading.value = true
  try {
    if (side.value === 'mine') mine.value = await libraryBooks()
    else {
      spiral.value = await spiralRoom()
      loadKept()
    }
  } catch (e) {
    problem.value = String(e?.message ?? e)
  } finally {
    loading.value = false
  }
}

async function readBook(b) {
  problem.value = ''
  reading.value = true
  try {
    if (side.value === 'mine') {
      open.value = { title: b.title, body: await libraryRead(b.id), id: b.id }
    } else {
      const book = await spiralRead(b.ref)
      open.value = { title: book.title, subtitle: book.subtitle, body: book.body, spiral: book }
    }
  } catch (e) {
    problem.value = String(e?.message ?? e)
  } finally {
    reading.value = false
  }
}

async function remove(id) {
  try {
    mine.value = await libraryDelete(id)
    asking.value = null
    if (open.value?.id === id) open.value = null
  } catch (e) {
    problem.value = String(e?.message ?? e)
  }
}

async function signIn() {
  try {
    await memberSignIn()
    await load()
  } catch (e) {
    problem.value = String(e?.message ?? e)
  }
}

watch(side, load)
watch(() => props.stamp, load)
onMounted(load)
</script>

<template>
  <div class="library">
    <header class="bar">
      <h2>LIBRARY</h2>
      <div class="switch" role="tablist" aria-label="Library side">
        <button role="tab" :aria-selected="side === 'mine'" :class="{ on: side === 'mine' }" @click="side = 'mine'">My books</button>
        <button role="tab" :aria-selected="side === 'spiral'" :class="{ on: side === 'spiral' }" @click="side = 'spiral'">Spiral Library</button>
      </div>
      <input v-model="query" class="search" type="search" placeholder="Search…" aria-label="Search the library" />
      <span class="spacer"></span>
      <span v-if="loading" class="busy">Reading…</span>
      <button v-if="side === 'spiral' && !spiral.signedIn" @click="signIn">Sign in with your Library card</button>
      <button @click="load">Refresh</button>
    </header>
    <p v-if="problem" class="problem" role="alert">{{ problem }}</p>

    <div class="room">
      <nav class="shelves widget" aria-label="Shelves">
        <button v-for="s in shelves" :key="s.key" :class="{ on: shelf === s.key }" @click="shelf = s.key">
          <span>{{ s.label }}</span><span class="count">{{ s.count }}</span>
        </button>
      </nav>

      <section class="cards widget">
        <p v-if="!loading && !books.length" class="empty">
          {{ side === 'mine' ? `Your library is empty. Ask ${companion} to save something worth keeping.` : 'The Spiral Library has no books to show right now.' }}
        </p>
        <p v-else-if="!shown.length && books.length" class="empty">No book matches.</p>
        <div class="grid">
          <article v-for="b in shown" :key="b.id ?? b.ref" class="card" :class="{ active: open && (open.id === b.id && b.id !== undefined || open.spiral?.ref === b.ref) }">
            <button class="open" @click="readBook(b)">
              <img v-if="b.coverUrl" class="cover" :src="b.coverUrl" :alt="b.title" loading="lazy" />
              <span v-if="b.origin" class="badge" :title="`Kept from the Spiral Library (${b.origin})`">Spiral</span>
              <strong>{{ b.title }}</strong>
              <span v-if="b.subtitle" class="sub">{{ b.subtitle }}</span>
              <span class="line">{{ b.excerpt ?? b.summary }}</span>
              <span class="meta">
                <template v-if="side === 'mine'">{{ b.words }} words</template>
                <template v-else>{{ b.readingMinutes ? `${b.readingMinutes} min` : `${b.wordCount} words` }} · {{ priceOf(b) }}</template>
              </span>
            </button>
            <template v-if="side === 'mine'">
              <span v-if="asking === b.id" class="confirm">Delete? <button class="danger" @click="remove(b.id)">Yes</button> <button @click="asking = null">No</button></span>
              <button v-else class="delete" :aria-label="`Delete ${b.title}`" title="Delete" @click="asking = b.id">✕</button>
            </template>
          </article>
        </div>
      </section>

      <section class="reader widget" aria-live="polite">
        <p v-if="reading" class="empty">Opening…</p>
        <template v-else-if="open">
          <h3>{{ open.title }}</h3>
          <p v-if="open.subtitle" class="sub">{{ open.subtitle }}</p>
          <p v-if="preview" class="preview">{{ preview }}</p>
          <p v-if="keep !== 'none'" class="keep">
            <button :class="{ primary: keep !== 'kept' }" :disabled="keep === 'kept' || keeping" @click="keepBook">
              {{ keeping ? 'Keeping…' : KEEP_WORDS[keep] }}
            </button>
            <span v-if="keep === 'update'" class="dim">The Spiral Library has a newer edition.</span>
          </p>
          <div class="text"><MarkdownText :text="open.body" size="reader" /></div>
        </template>
        <p v-else class="empty">Choose a book to read it here.</p>
      </section>
    </div>
  </div>
</template>

<style scoped>
.library {
  position: absolute;
  inset: 24px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.widget {
  background: var(--panel-background);
  border: 1px solid var(--line);
  border-radius: 14px;
  backdrop-filter: blur(var(--glass-blur));
}
.bar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 12px;
}
h2 {
  margin: 0;
  font-size: 13px;
  font-weight: 500;
  letter-spacing: 0.2em;
  color: var(--accent);
}
.switch {
  display: inline-flex;
  padding: 3px;
  border: 1px solid var(--line);
  border-radius: 999px;
}
.switch button {
  border: 0;
  border-radius: 999px;
  padding: 5px 16px;
}
.switch button.on {
  background: var(--accent);
  color: #07142e;
  font-weight: 600;
}
.search {
  min-width: 220px;
  padding: 7px 12px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: rgba(0, 0, 0, 0.25);
  color: var(--text);
  font: inherit;
  font-size: 13px;
}
.spacer {
  flex: 1;
}
.room {
  display: grid;
  grid-template-columns: 200px minmax(0, 1.2fr) minmax(0, 1fr);
  gap: 14px;
  flex: 1;
  min-height: 0;
}
.shelves,
.cards,
.reader {
  min-height: 0;
  overflow-y: auto;
  scrollbar-width: thin;
  scrollbar-color: var(--line) transparent;
}
.shelves {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 10px;
}
.shelves button {
  display: flex;
  justify-content: space-between;
  border: 0;
  text-align: left;
}
.shelves button.on {
  background: color-mix(in srgb, var(--accent) 22%, transparent);
  color: var(--accent);
}
.count {
  color: var(--text-dim);
  font-size: 12px;
}
.cards {
  padding: 14px;
}
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 12px;
}
.card {
  position: relative;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: color-mix(in srgb, var(--panel-background) 70%, transparent);
}
.card.active {
  border-color: var(--accent);
}
.card .open {
  display: flex;
  flex-direction: column;
  gap: 6px;
  width: 100%;
  padding: 12px;
  border: 0;
  text-align: left;
}
.cover {
  width: 100%;
  aspect-ratio: 3 / 4;
  object-fit: cover;
  border-radius: 8px;
}
.sub,
.line,
.meta {
  font-size: 12px;
  color: var(--text-dim);
}
.line {
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.delete {
  position: absolute;
  top: 6px;
  right: 6px;
  width: 26px;
  height: 26px;
  padding: 0;
  border-radius: 50%;
  opacity: 0;
}
.card:hover .delete,
.delete:focus-visible {
  opacity: 1;
}
.confirm {
  display: block;
  padding: 0 12px 10px;
  font-size: 13px;
}
.danger {
  border-color: color-mix(in srgb, #ff6b81 60%, var(--line));
  color: #ff9aa9;
}
.reader {
  padding: 18px 22px;
}
.reader h3 {
  margin: 0 0 4px;
  font-size: 18px;
  font-weight: 500;
}
.keep {
  display: flex;
  align-items: center;
  gap: 10px;
  margin: 10px 0;
}
.keep .primary {
  background: var(--accent);
  border-color: var(--accent);
  color: #07142e;
  font-weight: 600;
}
.keep button:disabled {
  opacity: 0.7;
  cursor: default;
}
.dim {
  font-size: 12px;
  color: var(--text-dim);
}
.badge {
  align-self: flex-start;
  padding: 1px 8px;
  border: 1px solid color-mix(in srgb, var(--accent) 60%, var(--line));
  border-radius: 999px;
  color: var(--accent);
  font-size: 11px;
  letter-spacing: 0.06em;
}
.preview {
  padding: 8px 12px;
  border: 1px solid color-mix(in srgb, var(--warn) 50%, var(--line));
  border-radius: 8px;
  color: var(--warn);
  font-size: 13px;
}
.empty {
  margin: 0;
  font-size: 13px;
  color: var(--text-dim);
}
.problem {
  margin: 0;
  font-size: 13px;
  color: var(--warn);
}
.busy {
  font-size: 13px;
  color: var(--accent);
}
button {
  padding: 6px 12px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 13px;
  cursor: pointer;
}
@media (max-width: 1000px) {
  .room { grid-template-columns: 160px minmax(0, 1fr); }
  .reader { grid-column: 1 / -1; min-height: 300px; }
}
</style>
