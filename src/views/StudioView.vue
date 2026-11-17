<script setup>
// STUDIO (the chosen design, studio-room-1): one Make bar - a prompt box, four powers with their
// state, and a main button that follows them - over a gallery that fills the room with everything
// the companion made. A picture goes to the queue (castle-image with the key, or Codex); a video, a
// song or a voice line is asked of the companion, which runs its own helper and tells the cost.
import { computed, onMounted, ref, watch } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import ImageViewer from '../components/ImageViewer.vue'
import ImageJobs from '../components/ImageJobs.vue'
import ThemedAudio from '../components/ThemedAudio.vue'
import { chatMediaOpen, imageRequest, studioDelete, voiceStudioSpeak } from '../api/castle.js'
import { refreshStudio, studioLib, studioPowersLoaded, studioPowersNow, studioThumbUrls, studioVoiceUp } from '../composables/useStudio.js'
import { askCompanion } from '../api/ask.js'
import { POWERS, companionLines, makeAction, powerPill } from '../components/studiomake.js'
import { isMac } from '../platform.js'

const props = defineProps({
  companion: { type: String, default: 'Your companion' },
  stamp: { type: Number, default: 0 },
})
const emit = defineEmits(['keys'])

// kept for the whole run and read at the start (composables/useStudio.js), so the room opens full
const powers = studioPowersNow
const voiceUp = studioVoiceUp
const lib = studioLib

// ---------- the Make bar ----------
const power = ref('image')
const powersLoaded = studioPowersLoaded
const brief = ref('')
const making = ref(false)
const makeProblem = ref('')
const makeDone = ref('')
const voiceClip = ref('')
const action = computed(() => makeAction(power.value, powers.value, voiceUp.value, props.companion, powersLoaded.value))
const placeholder = computed(() => ({ image: 'Describe the picture…', video: 'Describe the short video…', song: 'What should the song be about…', voice: 'Type the words to say…' })[power.value])
const canMake = computed(() => action.value.kind === 'key' || (action.value.kind !== 'checking' && !!brief.value.trim() && !making.value))

async function make() {
  const a = action.value
  makeProblem.value = ''
  makeDone.value = ''
  if (a.kind === 'checking') return
  if (a.kind === 'key') return emit('keys')
  if (!brief.value.trim() || making.value) return
  making.value = true
  try {
    if (a.kind === 'image') {
      await imageRequest(brief.value)
    } else if (a.kind === 'voicestudio') {
      voiceClip.value = convertFileSrc(await voiceStudioSpeak(brief.value))
      await load()
    } else {
      await askCompanion(companionLines(a.ask, brief.value), props.companion)
      makeDone.value = `Asked ${props.companion} - it tells you the cost in Chat and asks before it makes it.`
    }
    brief.value = ''
  } catch (e) {
    makeProblem.value = String(e?.message ?? e)
  } finally {
    making.value = false
  }
}

// ---------- the gallery ----------
const TABS = [
  ['images', 'Pictures'],
  ['videos', 'Videos'],
  ['songs', 'Songs'],
  ['voices', 'Voice'],
  ['saved', 'Saved prompts'],
  ['backups', 'Backups'],
]
const tab = ref('images')
const viewerStart = ref(null)
const src = (path) => convertFileSrc(path)
const counts = computed(() => Object.fromEntries(TABS.map(([t]) => [t, lib.value[t]?.length ?? 0])))
// the viewer walks the shelf in view: Pictures, or the Backups Castle kept
const shelfPictures = computed(() => (tab.value === 'backups' ? lib.value.backups : lib.value.images))
const pictures = computed(() => shelfPictures.value.map((m) => ({ path: m.path, name: m.name, url: src(m.path) })))

// Delete asks once, in place ("Delete? Yes / No"), then sends the file to the Recycle Bin.
const asking = ref('')
const deleteProblem = ref('')
async function remove(path) {
  deleteProblem.value = ''
  try {
    await studioDelete(path)
    asking.value = ''
    await load()
  } catch (e) {
    deleteProblem.value = String(e?.message ?? e)
  }
}

function when(at) {
  return at ? new Date(at * 1000).toLocaleString('en-US', { dateStyle: 'medium', timeStyle: 'short' }) : ''
}

// what is already read shows at once; this reads again quietly and swaps in anything new
const load = refreshStudio

onMounted(load)
watch(() => props.stamp, load)
</script>

<template>
  <div class="studio">
    <section class="make widget">
      <h3>MAKE</h3>
      <div class="make-row">
        <textarea v-model="brief" rows="2" :placeholder="placeholder" aria-label="What to make" :disabled="action.kind === 'key'" @keydown.ctrl.enter="make" @keydown.meta.enter="isMac && make()"></textarea>
        <div class="switch" role="radiogroup" aria-label="What to make">
          <button
            v-for="p in POWERS"
            :key="p.id"
            role="radio"
            :aria-checked="power === p.id"
            :class="{ on: power === p.id }"
            @click="power = p.id"
          >
            <span class="power-name">{{ p.label }}</span>
            <span class="pill" :class="powerPill(p.id, powers, voiceUp, powersLoaded).tone">{{ powerPill(p.id, powers, voiceUp, powersLoaded).text }}</span>
          </button>
        </div>
        <button class="primary create" :disabled="!canMake" @click="make">{{ making ? 'Working…' : action.label }}</button>
      </div>
      <p v-if="makeProblem" role="alert" class="problem">{{ makeProblem }}</p>
      <p v-else-if="makeDone" class="hint">{{ makeDone }}</p>
      <ThemedAudio v-if="voiceClip && power === 'voice'" :src="voiceClip" />
    </section>

    <section class="gallery widget">
      <nav class="tabs">
        <button v-for="[t, label] in TABS" :key="t" :class="{ active: tab === t }" @click="tab = t; asking = ''">{{ label }} ({{ counts[t] }})</button>
        <span class="spacer"></span>
        <ImageJobs @finished="load" />
        <button @click="load">Refresh</button>
      </nav>

      <p v-if="deleteProblem" role="alert" class="problem">{{ deleteProblem }}</p>
      <p v-if="tab === 'backups'" class="hint">Castle keeps the picture it replaces when it draws or you upload a new one. Delete the ones you no longer need; they go to the Recycle Bin.</p>
      <p v-if="tab === 'saved'" class="hint">These were saved instead of made, because a key was missing. Add the key and make them again above.</p>

      <div class="shelf">
        <div v-if="tab === 'images' || tab === 'backups'" class="masonry">
          <div v-for="(m, i) in shelfPictures" :key="m.path" class="tile">
            <button class="thumb" :title="m.name" @click="viewerStart = i">
              <img :src="studioThumbUrls[m.path] ?? src(m.path)" :alt="m.name" loading="lazy" />
            </button>
            <div v-if="asking === m.path" class="confirm" role="group" aria-label="Delete this picture">
              <span>Delete?</span>
              <button class="danger" @click="remove(m.path)">Yes</button>
              <button @click="asking = ''">No</button>
            </div>
            <button v-else class="delete" :title="`Delete ${m.name}`" :aria-label="`Delete ${m.name}`" @click="asking = m.path">✕</button>
          </div>
        </div>
        <div v-else class="rows">
          <div v-for="m in lib[tab]" :key="m.path" class="row" :class="tab">
            <video v-if="tab === 'videos'" :src="src(m.path)" controls preload="metadata"></video>
            <div class="row-name">
              <strong>{{ m.name }}</strong>
              <span v-if="m.group" class="group">{{ m.group }}</span>
              <span class="group">{{ when(m.at) }}</span>
            </div>
            <audio v-if="tab === 'songs' || tab === 'voices'" :src="src(m.path)" controls preload="none"></audio>
            <button v-if="tab === 'saved'" @click="chatMediaOpen(m.path, true)">Show file</button>
            <span v-if="asking === m.path" class="inline-confirm">Delete? <button class="danger" @click="remove(m.path)">Yes</button> <button @click="asking = ''">No</button></span>
            <button v-else class="text-delete" @click="asking = m.path">Delete</button>
          </div>
        </div>
        <p v-if="!counts[tab]" class="empty">Nothing here yet. Make one above.</p>
      </div>
    </section>

    <ImageViewer v-if="viewerStart !== null" :pictures="pictures" :start="viewerStart" @close="viewerStart = null" />
  </div>
</template>

<style scoped>
.studio {
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
  -webkit-backdrop-filter: blur(var(--glass-blur)); backdrop-filter: blur(var(--glass-blur));
}
h3 {
  margin: 0;
  font-size: 13px;
  font-weight: 500;
  letter-spacing: 0.18em;
}

/* the Make bar */
.make {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 16px 20px;
  flex-shrink: 0;
}
.make-row {
  display: flex;
  flex-wrap: wrap;
  align-items: stretch;
  gap: 14px;
}
.make-row .create {
  /* the label changes with the power and what it can do (Create picture, Ask <companion>, Add the
     ElevenLabs key, Working...): a fixed floor keeps the button, and the box beside it, from
     shifting width as you switch powers or it starts working */
  flex: 0 0 auto;
  min-width: 190px;
  white-space: nowrap;
}
.make-row textarea {
  /* fills what the fixed-width switch and create button leave behind, so it keeps one width as
     you move between Picture, Video, Song and Voice instead of resizing with them */
  flex: 1 1 320px;
  min-width: 260px;
  min-height: 56px;
  resize: vertical;
  padding: 10px 12px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.25);
  color: var(--text);
  font: inherit;
  font-size: 14px;
}
.switch {
  display: flex;
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 10px;
}
.switch button {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 4px;
  /* a fixed width, not a minimum: the pill below (Ready, Codex, VoiceStudio, Add a key) is a
     different length for each power, and a minimum let the button - and everything to its right -
     change width as you switched between them */
  width: 112px;
  flex-shrink: 0;
  padding: 8px 6px;
  border: 0;
  border-right: 1px solid var(--line);
  border-radius: 0;
}
.switch button:last-child {
  border-right: 0;
}
.switch button.on {
  background: color-mix(in srgb, var(--accent) 85%, transparent);
  color: #07142e;
}
.power-name {
  font-size: 14px;
  letter-spacing: 0.04em;
  white-space: nowrap;
}
.pill {
  padding: 1px 8px;
  border-radius: 999px;
  font-size: 10px;
  white-space: nowrap;
}
.pill.ready {
  background: color-mix(in srgb, var(--good) 30%, transparent);
  color: var(--good);
}
.switch button.on .pill.ready {
  background: #1f8f5f;
  color: #eafff4;
}
.pill.missing {
  background: color-mix(in srgb, var(--warn) 30%, transparent);
  color: var(--warn);
}
.switch button.on .pill.missing {
  background: #b07a1f;
  color: #fff6e6;
}
/* Picture's sign-in check still answering: neutral, not the "missing" colour, so a brief check
   never looks like a warning that something is actually wrong */
.pill.checking {
  background: color-mix(in srgb, var(--text-dim) 25%, transparent);
  color: var(--text-dim);
}
.switch button.on .pill.checking {
  background: color-mix(in srgb, #07142e 70%, transparent);
  color: #e8ecf4;
}
.create {
  min-width: 150px;
  font-size: 15px;
}

/* the gallery */
.gallery {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 10px;
  min-height: 0;
  padding: 14px 18px;
}
.tabs {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
}
.tabs > button {
  border-radius: 999px;
  padding: 6px 16px;
}
.tabs .active {
  border-color: var(--accent);
  background: var(--accent);
  color: #07142e;
  font-weight: 600;
}
.spacer {
  flex: 1;
}
.shelf {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  scrollbar-width: var(--scrollbar-width);
  scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track);
}
.masonry {
  columns: 280px;
  column-gap: 12px;
}
.tile {
  position: relative;
  break-inside: avoid;
  margin-bottom: 12px;
}
.thumb {
  display: block;
  width: 100%;
  padding: 0;
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 12px;
  cursor: zoom-in;
}
.thumb img {
  display: block;
  width: 100%;
  height: auto;
}
.delete {
  position: absolute;
  top: 8px;
  right: 8px;
  width: 28px;
  height: 28px;
  padding: 0;
  border-radius: 50%;
  border: 1px solid var(--line);
  background: color-mix(in srgb, var(--panel-background) 85%, transparent);
  color: var(--text);
  opacity: 0;
  cursor: pointer;
}
.tile:hover .delete,
.delete:focus-visible {
  opacity: 1;
}
.confirm {
  position: absolute;
  inset: auto 8px 8px 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 6px;
  border-radius: 8px;
  background: color-mix(in srgb, var(--panel-background) 92%, transparent);
  font-size: 13px;
}
.danger {
  border-color: color-mix(in srgb, #ff6b81 60%, var(--line));
  color: #ff9aa9;
}
.rows {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 12px;
  padding: 10px 14px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: color-mix(in srgb, var(--panel-background) 70%, transparent);
  font-size: 14px;
}
.row.videos video {
  width: 100%;
  max-width: 560px;
  border-radius: 10px;
}
.row-name {
  flex: 1;
  min-width: 180px;
}
.group {
  margin-left: 10px;
  font-size: 12px;
  color: var(--text-dim);
}
.inline-confirm {
  font-size: 13px;
}
.problem {
  margin: 0;
  font-size: 13px;
  color: var(--warn);
}
.hint,
.empty {
  margin: 0;
  font-size: 13px;
  color: var(--text-dim);
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
