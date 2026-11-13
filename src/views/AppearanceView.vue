<script setup>
// APPEARANCE: the companion's standard look, the wardrobe and the hairstyles as turnaround
// sheets, today's outfit and hair, and Try on. Drawing is the companion's work: a request goes to
// it with the right style file; with Codex it draws by itself, with Claude Code it needs the
// ChatGPT key, and without one the prompt is saved and Castle says so.
import { computed, onMounted, ref, watch } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import ImageViewer from '../components/ImageViewer.vue'
import ImageJobs from '../components/ImageJobs.vue'
import { appearanceImport, appearanceLook, appearanceNew, appearanceWear, choosePicture, studioPowers } from '../api/castle.js'
import { referenceLine, improvementLine } from '../components/appearance.js'
import { askImage as askCompanion } from '../api/ask.js'

const props = defineProps({
  companion: { type: String, default: 'Your companion' },
  stamp: { type: Number, default: 0 },
})
const emit = defineEmits(['keys', 'show-chat'])

const look = ref(null)
const image = ref('')
const busy = ref('')
const problem = ref('')
const done = ref('')
const form = ref({ kind: '', name: '', description: '' })
const viewer = ref(null)
const changes = ref('')
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

const STATE = { ready: 'Ready to draw', builtin: 'Codex available — checked when drawing', prompt: 'Codex unavailable — prompts are saved' }
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

function styleLine(file) {
  return `Use the drawing style in %CASTLE_STYLES%\\${file} and fill its slots from ${path('companion.md')}.`
}

async function drawPortrait() {
  const label = 'Standard look'
  const ok = await attempt('Asking…', () =>
    askCompanion(
      [
        `Please draw your standard appearance for Castle's APPEARANCE room.`,
        styleLine('standard-appearance.md'),
        referenceLine(look.value.portrait),
        improvementLine(changes.value),
        'Keep the original reference unchanged; preserve a backup before replacing any existing output.',
        `Save it as ${path('portrait.png')}.`,
        'Tell me the cost first and ask before you draw.',
      ],
      label,
    ),
  )
  if (ok) after(label)
}

async function drawSheet(kind, item) {
  const label = `${kind === 'outfit' ? 'Outfit' : 'Hairstyle'} turnaround: ${item.name}`
  const dir = kind === 'outfit' ? 'wardrobe' : 'hairstyles'
  const note = kind === 'outfit' ? 'outfit.md' : 'hair.md'
  const style = kind === 'outfit' ? 'outfit-turnaround.md' : 'hair-turnaround.md'
  const ok = await attempt('Asking…', () =>
    askCompanion(
      [
        `Please draw the ${kind === 'outfit' ? 'outfit' : 'hairstyle'} turnaround (front, side, back) for "${item.name}".`,
        styleLine(style),
        `The ${kind === 'outfit' ? 'outfit' : 'hairstyle'} is described in ${path(dir, item.id, note)}.`,
        referenceLine(item.reference || item.sheet || look.value.portrait),
        improvementLine(changes.value),
        look.value.portrait ? `Also inspect the standard look for identity: ${JSON.stringify(look.value.portrait)}.` : 'Keep the face as companion.md describes it.',
        'Keep imported references unchanged and preserve a backup before replacing any existing output.',
        `Save it as ${path(dir, item.id, 'turnaround.png')}.`,
        'Tell me the cost first and ask before you draw.',
      ],
      label,
    ),
  )
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
  const ok = await attempt('Asking…', () =>
    askCompanion(
      [
        `Please draw a try-on picture: you in ${outfit ? `the outfit "${outfit.name}" (${path('wardrobe', outfit.id, 'outfit.md')})` : 'your usual clothes'} with ${hair ? `the hairstyle "${hair.name}" (${path('hairstyles', hair.id, 'hair.md')})` : 'your usual hair'}.`,
        styleLine('try-on.md'),
        referenceLine(look.value.portrait),
        outfit?.reference ? `Inspect outfit reference: ${JSON.stringify(outfit.reference)}.` : '',
        hair?.reference ? `Inspect hairstyle reference: ${JSON.stringify(hair.reference)}.` : '',
        improvementLine(changes.value),
        `Save it as ${saved.path}.`,
        'Tell me the cost first and ask before you draw.',
      ].filter(Boolean),
      label,
    ),
  )
  if (ok) after(label)
}

function after(label) {
  done.value = `Queued for Codex: ${label}. Watch it under Image requests; the picture appears here when it is done.`
}

// Adds the outfit or hairstyle folder with its words, then asks the companion to draw its sheet.
async function create() {
  const kind = form.value.kind
  let item = null
  const ok = await attempt('Adding…', async () => {
    const id = await appearanceNew(kind, form.value.name, form.value.description)
    if (formPicture.value) await appearanceImport(kind, id, formPicture.value)
    await load()
    item = (kind === 'outfit' ? look.value.wardrobe : look.value.hairstyles).find((i) => i.id === id)
    form.value = { kind: '', name: '', description: '' }
    formPicture.value = ''
  })
  if (ok && item) done.value = 'Added. You can wear this item or draw its turnaround when ready.'
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
      <button @click="load">Refresh</button>
    </header>

    <label class="edit-request">Changes for the next drawing
      <textarea v-model="changes" rows="2" placeholder="For example: keep the face and proportions, change only the jacket colour." :disabled="!!busy"></textarea>
    </label>

    <ImageJobs @finished="load" />
    <div v-if="look" class="columns">
      <section class="standard widget">
        <h3>STANDARD LOOK</h3>
        <button v-if="look.portrait" class="portrait" @click="openSheet(look.portrait, 'Standard look')">
          <img :src="src(look.portrait)" alt="Standard look" />
        </button>
        <div v-else class="portrait empty"><span class="crystal">◆</span></div>
        <button :disabled="!!busy" @click="upload('standard')">{{ look.portrait ? 'Replace image' : 'Upload image' }}</button>
        <button class="primary" :disabled="!!busy" @click="drawPortrait">{{ look.portrait ? 'Improve from image' : 'Draw the standard look' }}</button>
        <button v-if="look.tryOn?.exists" class="sheet saved" @click="openSheet(look.tryOn.path, 'Today\'s look')"><img :src="src(look.tryOn.path)" alt="Today's look" /></button>
        <p v-if="tryOnLine" class="hint">{{ tryOnLine }}</p>
        <button :disabled="!!busy" @click="tryOn()">{{ !look.tryOn?.exists || look.tryOn.stale ? "Try on today's look" : "Show today's look" }}</button>
        <button v-if="look.tryOn?.exists && !look.tryOn.stale" :disabled="!!busy" @click="tryOn(true)">Draw again</button>
        <p class="hint">Upload PNG, JPEG or WebP (up to 25 MB). Drawing uses Codex separately from your conversation. If unavailable, the prompt is saved; there is no automatic API fallback.</p>
        <p class="hint">The look in words lives in <code>appearance/companion.md</code> - ask {{ companion }} to change it, or edit it yourself.</p>
      </section>

      <section v-for="shelf in [{ kind: 'outfit', title: 'WARDROBE', items: look.wardrobe, current: look.current.outfit }, { kind: 'hair', title: 'HAIR', items: look.hairstyles, current: look.current.hair }]" :key="shelf.kind" class="shelf widget">
        <header class="shelf-head">
          <h3>{{ shelf.title }}</h3>
          <button :disabled="!!busy" @click="newForm(shelf.kind)">+ New</button>
        </header>
        <div v-if="form.kind === shelf.kind" class="new">
          <label>Name<input v-model="form.name" :placeholder="shelf.kind === 'outfit' ? 'Winter coat' : 'Twin tails'" maxlength="40" /></label>
          <label>Description<input v-model="form.description" :placeholder="shelf.kind === 'outfit' ? 'a long navy coat with silver buttons, a white scarf' : 'high twin tails tied with blue ribbons'" /></label>
          <button :disabled="!!busy" @click="pickFormPicture">{{ formPicture ? 'Change reference image' : 'Add reference image' }}</button>
          <span v-if="formPicture" class="hint">{{ formPicture.split(/[\\/]/).pop() }}</span>
          <div class="row">
            <button class="primary" :disabled="!form.name.trim() || !!busy" @click="create">Add</button>
            <button :disabled="!!busy" @click="newForm('')">Cancel</button>
          </div>
        </div>
        <p v-if="!shelf.items.length && form.kind !== shelf.kind" class="hint">Nothing here yet. Press + New and describe it.</p>
        <div class="items">
          <article v-for="item in shelf.items" :key="item.id" class="item" :class="{ worn: shelf.current === item.id }">
            <button v-if="item.sheet" class="sheet" @click="openSheet(item.sheet, item.name)"><img :src="src(item.sheet)" :alt="item.name" /></button>
            <button v-else-if="item.reference" class="sheet" @click="openSheet(item.reference, item.name + ' reference')"><img :src="src(item.reference)" :alt="item.name + ' reference'" /></button>
            <div v-else class="sheet empty">{{ item.savedPrompt ? 'Prompt saved - add the key to draw' : 'Not drawn yet' }}</div>
            <strong>{{ item.name }}</strong>
            <p class="hint">{{ item.note }}</p>
            <div class="row">
              <button :disabled="!!busy" :class="{ primary: shelf.current !== item.id }" @click="wear(shelf.kind, item.id)">{{ shelf.current === item.id ? 'Wearing' : 'Wear' }}</button>
              <button :disabled="!!busy" @click="upload(shelf.kind, item.id)">{{ item.reference ? 'Replace reference' : 'Upload reference' }}</button>
              <button :disabled="!!busy" @click="drawSheet(shelf.kind, item)">{{ item.reference || item.sheet ? 'Improve / draw turnaround' : 'Draw' }}</button>
            </div>
          </article>
        </div>
      </section>
    </div>
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
}
.widget {
  background: var(--panel-background);
  border: 1px solid var(--line);
  border-radius: 14px;
  backdrop-filter: blur(var(--glass-blur));
}
.bar,
.row,
.shelf-head {
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
h3 {
  margin: 0;
  font-size: 13px;
  font-weight: 500;
  letter-spacing: 0.14em;
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
.columns {
  display: grid;
  grid-template-columns: 300px 1fr 1fr;
  gap: 16px;
  flex: 1;
  min-height: 0;
}
.standard,
.shelf {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 16px;
  overflow-y: auto;
}
.shelf-head {
  justify-content: space-between;
}
.portrait {
  padding: 0;
  overflow: hidden;
  aspect-ratio: 2 / 3;
  border-radius: 12px;
}
.portrait img,
.sheet img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.portrait.empty {
  display: grid;
  place-items: center;
  border: 1px dashed var(--line);
  background: radial-gradient(circle at 50% 40%, #1d3b8a, transparent 70%);
}
.crystal {
  font-size: 64px;
  color: #cfe0ff;
  text-shadow: 0 0 24px #7aa6ff;
}
.items {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 12px;
}
.item {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 10px;
  border: 1px solid var(--line);
  border-radius: 12px;
}
.item.worn {
  border-color: var(--accent);
}
.sheet {
  padding: 0;
  overflow: hidden;
  aspect-ratio: 3 / 2;
  border-radius: 8px;
}
.sheet.empty {
  display: grid;
  place-items: center;
  padding: 8px;
  border: 1px dashed var(--line);
  font-size: 12px;
  color: var(--text-dim);
  text-align: center;
}
.new {
  display: flex;
  flex-direction: column;
  gap: 8px;
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
}
textarea { resize: vertical; min-height: 48px; }
.edit-request { flex-shrink: 0; }
@media (max-width: 900px) {
  .appearance { overflow-y: auto; }
  .columns { grid-template-columns: 1fr; flex: none; }
  .portrait { max-height: 400px; }
}
.hint {
  margin: 0;
  font-size: 12px;
  color: var(--text-dim);
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
  padding: 6px 12px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 13px;
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
