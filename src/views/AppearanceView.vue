<script setup>
// APPEARANCE: the companion's standard look, the wardrobe and the hairstyles as turnaround
// sheets, today's outfit and hair, and Try on. A drawing is queued by its parts and castle-look.ps1
// writes its prompt from the notes: Codex draws it on the ChatGPT plan, or Castle draws it with the
// OpenAI key; with neither the prompt is saved and Castle says so. No AI reads the notes for it.
import GlassSelect from '../components/GlassSelect.vue'
import { computed, onMounted, ref, watch } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import ImageViewer from '../components/ImageViewer.vue'
import ImageJobs from '../components/ImageJobs.vue'
import ClosetCard from '../components/ClosetCard.vue'
import HelpTip from '../components/HelpTip.vue'
import { appearanceImport, appearanceLook, appearanceNew, appearanceSaveItem, appearanceSaveLook, appearanceWear, choosePicture, imageRequest, studioPowers } from '../api/castle.js'
import { closetShelf, lookSpec, pictureCost, QUALITIES } from '../components/appearance.js'

const props = defineProps({
  companion: { type: String, default: 'Your companion' },
  stamp: { type: Number, default: 0 },
})
const emit = defineEmits(['keys'])

const look = ref(null)
const image = ref('')
const busy = ref('')
const problem = ref('')
const done = ref('')
const form = ref({ kind: '', name: '', description: '' })
// The look in words and its art style, as typed (kept in the memory database on Save).
const lookWords = ref('')
const lookStyle = ref('')
watch(
  () => [look.value?.words, look.value?.artStyle],
  ([words, style]) => {
    lookWords.value = words ?? ''
    lookStyle.value = style ?? ''
  },
)
const lookChanged = computed(() => !!look.value && (lookWords.value !== (look.value.words ?? '') || lookStyle.value !== (look.value.artStyle ?? '')))
async function saveLook() {
  const saved = await attempt('Saving…', async () => {
    look.value = await appearanceSaveLook(lookWords.value, lookStyle.value)
  })
  if (saved) done.value = 'The look is saved in your memory.'
}
// New name or words for one outfit or hairstyle (edited in place on its closet card).
async function saveItem(kind, id, { name, words }) {
  await attempt('Saving…', async () => {
    look.value = await appearanceSaveItem(kind, id, name, words)
  })
}
const viewer = ref(null)
const changes = ref('')
// The quality chosen for every drawing here, remembered on this computer only.
const QUALITY_KEY = 'castle.appearance.quality'
function rememberedQuality() {
  try {
    const saved = localStorage.getItem(QUALITY_KEY)
    return QUALITIES.some((q) => q.id === saved) ? saved : 'style'
  } catch {
    return 'style'
  }
}
const quality = ref(rememberedQuality())
// The closet shelf in view (Outfits or Hairstyles), remembered on this computer only.
const SHELF_KEY = 'castle.appearance.shelf'
function rememberedShelf() {
  try {
    return localStorage.getItem(SHELF_KEY) === 'hair' ? 'hair' : 'outfit'
  } catch {
    return 'outfit'
  }
}
const shelfKind = ref(rememberedShelf())
watch(shelfKind, (k) => {
  try {
    localStorage.setItem(SHELF_KEY, k)
  } catch {
    // the choice still holds until Castle closes
  }
})
const shelf = computed(() => closetShelf(look.value, shelfKind.value))
// What the (?) beside the look explains, in place of a paragraph on the page.
const helpText = computed(
  () =>
    `Castle writes each drawing's prompt from these words itself, so no AI tokens are spent. With Claude Code, Castle draws with your OpenAI key at the price on the button; with Codex, on your ChatGPT plan when Codex is signed in, otherwise with your key. Kept in your memory database; you can also ask ${props.companion} to change it. Uploads: PNG, JPEG or WebP up to 25 MB.`,
)
watch(quality, (q) => {
  try {
    localStorage.setItem(QUALITY_KEY, q)
  } catch {
    // the choice still holds until Castle closes
  }
})
// The price shows only when the next picture is paid with the key; on the ChatGPT plan it is not.
const qualityLabel = (q) => (q.usd !== null && image.value === 'ready' ? `${q.label} - about $${q.usd.toFixed(2)}` : q.label)
const revision = ref(0)
const formPicture = ref('')
const src = (path) => convertFileSrc(path) + `?v=${props.stamp}-${revision.value}`

// What a request is for, named the way the member sees it: which outfit, which hairstyle.
const nameOf = (list, id, usual) => list.find((i) => i.id === id)?.name ?? usual
const worn = computed(() => ({
  outfit: nameOf(look.value?.wardrobe ?? [], look.value?.current?.outfit, 'usual clothes'),
  hair: nameOf(look.value?.hairstyles ?? [], look.value?.current?.hair, 'usual hair'),
}))
const tryOnLine = computed(() => {
  const saved = look.value?.tryOn
  if (!saved) return ''
  const what = `${worn.value.outfit} + ${worn.value.hair}`
  if (!saved.exists) return `${what}: not drawn yet.`
  return saved.stale ? `${what}: saved, but the outfit, hair or standard look changed since - draw it again.` : `${what}: saved - no need to draw it again.`
})

const slideIndex = ref(0)
const slides = computed(() => {
  const list = []
  if (look.value?.portrait) list.push({ label: 'Standard look', path: look.value.portrait })
  if (look.value?.tryOn?.exists) list.push({ label: "Today's look", path: look.value.tryOn.path })
  return list
})
const slide = computed(() => slides.value[Math.min(slideIndex.value, slides.value.length - 1)] ?? null)
const step = (direction) => {
  const count = slides.value.length
  if (count) slideIndex.value = (slideIndex.value + direction + count) % count
}
// A try-on picture that was not there before is shown straight away; otherwise the index is kept
// inside the list when a picture goes away.
watch(
  () => slides.value.map((s) => s.path).join('|'),
  (now, before) => {
    const today = slides.value.findIndex((s) => s.path === look.value?.tryOn?.path)
    if (today >= 0 && before && !before.split('|').includes(slides.value[today].path)) slideIndex.value = today
    else if (slideIndex.value >= slides.value.length) slideIndex.value = 0
  },
)

const STATE = { ready: 'Ready - drawn with your OpenAI key', builtin: 'Codex on your ChatGPT plan', prompt: 'Prompt only - add an OpenAI key or sign in to Codex' }
const sep = computed(() => (look.value?.folder?.includes('/') ? '/' : '\\'))
const path = (...parts) => [look.value.folder, ...parts].join(sep.value)

async function load() {
  try {
    look.value = await appearanceLook()
    revision.value++
    image.value = (await studioPowers()).image
  } catch (reason) {
    problem.value = String(reason)
  }
}

async function attempt(label, work) {
  problem.value = ''
  done.value = ''
  busy.value = label
  try {
    await work()
    return true
  } catch (reason) {
    problem.value = String(reason?.message ?? reason)
    return false
  } finally {
    busy.value = ''
  }
}

// A drawing is queued by its parts; castle-look.ps1 writes the prompt from the member's notes, so
// no AI reads them.
async function queueLook(label, spec, output, reference) {
  await imageRequest(label, output, reference ? [reference] : [], 'appearance', label, spec)
}
const specFor = (kind, items = {}) => lookSpec(kind, { ...items, changes: changes.value, quality: quality.value })
// The price on a Draw button, only when the key pays.
// The small price chip on a card's Draw icon, only when the key pays.
const priceTag = (kind) => {
  const usd = pictureCost(kind, quality.value)
  return image.value === 'ready' && usd !== null ? `$${usd.toFixed(2)}` : ''
}
const priced = (text, kind) => {
  const usd = pictureCost(kind, quality.value)
  return image.value === 'ready' && usd !== null ? `${text} · about $${usd.toFixed(2)}` : text
}

// Redraw draws the standard look again from the words in companion.md alone; Improve from image
// gives the current portrait as the reference and changes only what is asked. While companion.md
// still has Castle's starting words, the portrait is the only description there is, so Redraw
// follows it too.
const followsPortrait = computed(() => !!look.value?.portrait && !look.value?.words?.trim())
async function drawPortrait(fromImage = false) {
  const label = fromImage ? 'Standard look (improved)' : 'Standard look'
  const reference = fromImage || followsPortrait.value ? look.value.portrait : null
  const ok = await attempt('Asking…', () => queueLook(label, specFor('standard'), path('portrait.png'), reference))
  if (ok) after(label)
}

// The single reference is always who the companion is: the standard look, or this item's own
// earlier turnaround when there is no standard look yet. An imported outfit or hairstyle picture
// may show someone else, so it is never the reference; the item's words describe the clothes or hair.
async function drawSheet(kind, item) {
  const label = `${kind === 'outfit' ? 'Outfit' : 'Hairstyle'} turnaround: ${item.name}`
  const dir = kind === 'outfit' ? 'wardrobe' : 'hairstyles'
  const identity = look.value.portrait || item.sheet
  const spec = specFor(kind, kind === 'outfit' ? { outfit: item.id } : { hair: item.id })
  const ok = await attempt('Asking…', () => queueLook(label, spec, path(dir, item.id, 'turnaround.png'), identity))
  if (ok) after(label)
}

// A picture already drawn for this outfit and hairstyle is shown, not drawn again - unless one of
// them (or the standard look) changed since, or `again` is true.
async function tryOn(again = false) {
  const outfit = look.value.wardrobe.find((w) => w.id === look.value.current.outfit)
  const hair = look.value.hairstyles.find((h) => h.id === look.value.current.hair)
  if (!outfit && !hair) {
    problem.value = 'Choose an outfit or a hairstyle to wear first.'
    return
  }
  const saved = look.value.tryOn
  const label = `Try-on: ${worn.value.outfit} + ${worn.value.hair}`
  if (saved?.exists && !saved.stale && again !== true) {
    openSheet(saved.path, label)
    done.value = 'Already drawn for this outfit and hairstyle - showing the saved picture. Use Draw again for a new one.'
    return
  }
  const spec = specFor('tryon', { outfit: outfit?.id ?? null, hair: hair?.id ?? null })
  const ok = await attempt('Asking…', () => queueLook(label, spec, saved.path, look.value.portrait))
  if (ok) after(label)
}

// The request line in the bar says where the picture went and how it is doing.
function after() {
  done.value = ''
}

// Adds the outfit or hairstyle folder with its words and, when one was chosen, its reference image.
async function create() {
  const kind = form.value.kind
  const name = form.value.name.trim()
  const picture = formPicture.value
  let importProblem = ''
  const ok = await attempt('Adding…', async () => {
    const id = await appearanceNew(kind, form.value.name, form.value.description)
    // The item exists from here on, so the form closes: a second Add would make a "-2" copy.
    form.value = { kind: '', name: '', description: '' }
    formPicture.value = ''
    if (picture) {
      try {
        await appearanceImport(kind, id, picture)
      } catch (reason) {
        importProblem = String(reason?.message ?? reason)
      }
    }
    await load()
  })
  if (!ok) return
  if (importProblem) problem.value = `"${name}" was added, but its reference image could not be imported: ${importProblem} Use Upload reference on it to try again.`
  else done.value = 'Added. You can wear this item or draw its turnaround when ready.'
}

async function upload(kind, id = null) {
  await attempt('Importing…', async () => {
    const selected = await choosePicture('Choose an existing appearance image')
    if (!selected) return
    look.value = await appearanceImport(kind, id, selected)
    revision.value++
    done.value = 'Image imported. No generation or API charge was started.'
  })
}

async function pickFormPicture() {
  await attempt('Choosing…', async () => {
    const selected = await choosePicture('Choose an outfit or hairstyle reference')
    if (selected) formPicture.value = selected
  })
}

function newForm(kind) {
  form.value = { kind, name: '', description: '' }
  formPicture.value = ''
}

async function wear(kind, id) {
  const current = look.value.current
  const next = { outfit: current.outfit, hair: current.hair }
  next[kind] = next[kind] === id ? null : id
  await attempt('Saving…', async () => {
    look.value = await appearanceWear(next.outfit, next.hair)
    done.value = 'Today\'s look is saved - your companion reads it from current.json.'
  })
}

function openSheet(file, name) {
  viewer.value = [{ path: file, name, url: src(file) }]
}

onMounted(load)
watch(() => props.stamp, load)
</script>

<template>
  <div class="appearance">
    <header class="bar">
      <h2>APPEARANCE</h2>
      <span class="state" :class="image">{{ STATE[image] ?? '…' }}</span>
      <span class="spacer"></span>
      <span v-if="busy" class="busy">{{ busy }}</span>
      <span v-else-if="problem" class="problem">{{ problem }}</span>
      <span v-else-if="done" class="good">{{ done }}</span>
      <ImageJobs @finished="load" />
      <button @click="load">Refresh</button>
    </header>

    <template v-if="look">
      <!-- the profile card: the standard look beside the look in words -->
      <section class="profile widget">
        <div class="frame">
          <div class="carousel" @keydown.left.prevent="step(-1)" @keydown.right.prevent="step(1)">
            <button v-if="slide" class="portrait" @click="openSheet(slide.path, slide.label)">
              <img :src="src(slide.path)" :alt="slide.label" />
            </button>
            <div v-else class="portrait empty"><span class="crystal">◆</span></div>
            <template v-if="slides.length > 1">
              <button class="nav prev" aria-label="Previous picture" @click="step(-1)">‹</button>
              <button class="nav next" aria-label="Next picture" @click="step(1)">›</button>
            </template>
            <button class="replace" :disabled="!!busy" :title="look.portrait ? 'Replace image' : 'Upload image'" @click="upload('standard')">{{ look.portrait ? 'Replace' : 'Upload' }}</button>
          </div>
          <p v-if="slide" class="hint caption">{{ slide.label }}<template v-if="slides.length > 1"> · {{ slideIndex + 1 }}/{{ slides.length }}</template></p>
        </div>

        <div class="look-words">
          <label class="title">THE LOOK IN WORDS
            <HelpTip :text="helpText" />
            <textarea v-model="lookWords" rows="6" maxlength="4000" placeholder="Face, eyes, hair colour, build, age look, and anything that always stays the same." :disabled="!!busy"></textarea>
          </label>
          <p v-if="followsPortrait" class="hint">No words yet, so Redraw follows the picture.</p>
        </div>

        <div class="look-side">
          <label class="title">ART STYLE
            <input v-model="lookStyle" maxlength="120" placeholder="anime, soft watercolor illustration..." :disabled="!!busy" />
          </label>
          <div class="actions">
            <button class="primary" :disabled="!lookChanged || !!busy" @click="saveLook">Save the look</button>
            <button :disabled="!!busy" @click="drawPortrait(false)">{{ priced(look.portrait ? 'Redraw' : 'Draw the standard look', 'standard') }}</button>
            <button v-if="look.portrait" :disabled="!!busy" @click="drawPortrait(true)">{{ priced('Improve from image', 'standard') }}</button>
          </div>
          <div class="actions">
            <button :disabled="!!busy" @click="tryOn()">{{ !look.tryOn?.exists || look.tryOn.stale ? "Try on today's look" : "Show today's look" }}</button>
            <button v-if="look.tryOn?.exists && !look.tryOn.stale" :disabled="!!busy" @click="tryOn(true)">{{ priced('Draw again', 'tryon') }}</button>
          </div>
          <p v-if="tryOnLine" class="hint">{{ tryOnLine }}</p>
        </div>
      </section>

      <!-- the drawing bar: how the next picture is drawn -->
      <section class="drawing widget">
        <label class="inline">QUALITY
          <GlassSelect v-model="quality" :options="QUALITIES.map((q) => ({ value: q.id, label: qualityLabel(q) }))" :disabled="!!busy" aria-label="Picture quality" />
        </label>
        <span class="divider" aria-hidden="true"></span>
        <label class="inline grow">CHANGES FOR THE NEXT DRAWING
          <input v-model="changes" placeholder="For example: keep the face and proportions, change only the jacket colour." :disabled="!!busy" />
        </label>
      </section>

      <!-- the closet: outfits or hairstyles, as large cards -->
      <section class="closet widget">
        <header class="closet-head">
          <h3>CLOSET</h3>
          <span class="spacer"></span>
          <div class="switch" role="tablist" aria-label="Closet shelf">
            <button role="tab" :aria-selected="shelfKind === 'outfit'" :class="{ on: shelfKind === 'outfit' }" @click="shelfKind = 'outfit'">Outfits</button>
            <button role="tab" :aria-selected="shelfKind === 'hair'" :class="{ on: shelfKind === 'hair' }" @click="shelfKind = 'hair'">Hairstyles</button>
          </div>
          <button :disabled="!!busy" @click="newForm(shelf.kind)">+ New</button>
        </header>
        <div class="cards">
          <div v-if="form.kind === shelf.kind" class="new">
            <label>Name<input v-model="form.name" :placeholder="shelf.kind === 'outfit' ? 'Winter coat' : 'Twin tails'" maxlength="40" /></label>
            <label>Description<textarea v-model="form.description" rows="3" :placeholder="shelf.kind === 'outfit' ? 'a long navy coat with silver buttons, a white scarf' : 'high twin tails tied with blue ribbons'"></textarea></label>
            <button :disabled="!!busy" @click="pickFormPicture">{{ formPicture ? 'Change reference image' : 'Add reference image' }}</button>
            <span v-if="formPicture" class="hint">{{ formPicture.split(/[\\/]/).pop() }}</span>
            <div class="actions">
              <button class="primary" :disabled="!form.name.trim() || !!busy" @click="create">Add</button>
              <button :disabled="!!busy" @click="newForm('')">Cancel</button>
            </div>
          </div>
          <ClosetCard
            v-for="item in shelf.items"
            :key="item.id"
            :item="item"
            :worn="shelf.worn === item.id"
            :busy="!!busy"
            :draw-label="priced(item.reference || item.sheet ? 'Improve / draw turnaround' : 'Draw turnaround', shelf.kind)"
            :price="priceTag(shelf.kind)"
            :stamp="`${stamp}-${revision}`"
            @wear="wear(shelf.kind, item.id)"
            @upload="upload(shelf.kind, item.id)"
            @draw="drawSheet(shelf.kind, item)"
            @save="(w) => saveItem(shelf.kind, item.id, w)"
            @view="openSheet(item.sheet || item.reference, item.name)"
          />
        </div>
        <p v-if="!shelf.items.length && form.kind !== shelf.kind" class="hint">{{ shelf.empty }}</p>
      </section>
    </template>
    <p v-else-if="problem" class="problem">{{ problem }}</p>

    <ImageViewer v-if="viewer" :pictures="viewer" :start="0" @close="viewer = null" />
  </div>
</template>

<style scoped>
.appearance {
  position: absolute;
  inset: 24px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow-y: auto;
}
.widget {
  background: var(--panel-background);
  border: 1px solid var(--line);
  border-radius: 14px;
  backdrop-filter: blur(var(--glass-blur));
}
.bar,
.actions,
.closet-head {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  align-items: center;
}
.spacer {
  flex: 1;
}
h2 {
  margin: 0;
  font-size: 13px;
  font-weight: 500;
  letter-spacing: 0.2em;
  color: var(--accent);
}
h3,
.title,
.inline {
  margin: 0;
  font-size: 13px;
  font-weight: 500;
  letter-spacing: 0.18em;
  color: var(--text);
}
.state {
  padding: 3px 10px;
  border-radius: 999px;
  font-size: 12px;
  background: color-mix(in srgb, var(--good) 22%, transparent);
  color: var(--good);
}
.state.prompt {
  background: color-mix(in srgb, var(--warn) 22%, transparent);
  color: var(--warn);
}

/* the profile card */
.profile {
  display: grid;
  grid-template-columns: 240px minmax(0, 1.4fr) minmax(0, 1fr);
  gap: 24px;
  padding: 20px;
  flex-shrink: 0;
}
.frame {
  display: grid;
  gap: 6px;
}
.carousel {
  position: relative;
}
.portrait {
  display: block;
  width: 100%;
  padding: 0;
  overflow: hidden;
  aspect-ratio: 3 / 4;
  border: 1px solid var(--line);
  border-radius: 14px;
}
.portrait img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.portrait.empty {
  display: grid;
  place-items: center;
  border-style: dashed;
  background: radial-gradient(circle at 50% 40%, #1d3b8a, transparent 70%);
}
.crystal {
  font-size: 64px;
  color: #cfe0ff;
  text-shadow: 0 0 24px #7aa6ff;
}
.carousel .nav {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  width: 30px;
  height: 30px;
  padding: 0;
  border-radius: 50%;
  font-size: 18px;
  line-height: 1;
  background: color-mix(in srgb, var(--panel-background) 80%, transparent);
}
.carousel .prev { left: 6px; }
.carousel .next { right: 6px; }
.replace {
  position: absolute;
  right: 8px;
  bottom: 8px;
  padding: 4px 10px;
  border-radius: 999px;
  font-size: 12px;
  background: color-mix(in srgb, var(--panel-background) 85%, transparent);
}
.caption { text-align: center; }
.look-words,
.look-side {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-width: 0;
}
.look-words .title textarea {
  flex: 1;
  min-height: 150px;
}
.title {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

/* the drawing bar */
.drawing {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 16px;
  padding: 12px 20px;
  flex-shrink: 0;
}
.inline {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-direction: row;
}
.inline.grow {
  flex: 1;
  min-width: 320px;
}
.inline.grow input {
  flex: 1;
}
.divider {
  width: 1px;
  align-self: stretch;
  background: var(--line);
}

/* the closet */
.closet {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 20px;
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
  padding: 5px 18px;
}
.switch button.on {
  background: var(--accent);
  color: #07142e;
  font-weight: 600;
}
.cards {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
  gap: 16px;
}
.new {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 14px;
  border: 1px dashed var(--line);
  border-radius: 14px;
}

label { display: flex; flex-direction: column; gap: 6px; font-size: 13px; }
input,
textarea {
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.25);
  color: var(--text);
  font: inherit;
  font-size: 14px;
  letter-spacing: normal;
}
textarea { resize: vertical; min-height: 48px; }
@media (max-width: 1100px) {
  .profile { grid-template-columns: 200px minmax(0, 1fr); }
  .look-side { grid-column: 1 / -1; }
}
@media (max-width: 700px) {
  .profile { grid-template-columns: 1fr; }
  .frame { max-width: 280px; }
}
.hint {
  margin: 0;
  font-size: 12px;
  color: var(--text-dim);
  letter-spacing: normal;
  font-weight: 400;
}
.busy {
  font-size: 13px;
  color: var(--accent);
}
.problem {
  font-size: 13px;
  color: var(--warn);
}
.good {
  font-size: 13px;
  color: var(--good);
}
button {
  padding: 7px 14px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 13px;
  letter-spacing: normal;
  cursor: pointer;
}
button.primary {
  background: var(--accent);
  border-color: var(--accent);
  color: #07142e;
  font-weight: 600;
}
button:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
</style>
