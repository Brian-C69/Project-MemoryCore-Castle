<script setup>
// STUDIO: the four powers (picture, video, song, voice) with whether each is ready, built in
// (Codex draws by itself) or saves a prompt (no key yet), and everything the companion made.
import { computed, onMounted, ref, watch } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import ImageViewer from '../components/ImageViewer.vue'
import ImageJobs from '../components/ImageJobs.vue'
import ThemedAudio from '../components/ThemedAudio.vue'
import { imageRequest } from '../api/castle.js'
const imageBrief = ref('')
const imageProblem = ref('')
// The picture goes the way Castle can draw now (env::picture_way); its progress shows in the
// request line beside the shelves' Refresh.
async function drawImage() {
  imageProblem.value = ''
  try {
    await imageRequest(imageBrief.value)
    imageBrief.value = ''
  } catch (e) {
    imageProblem.value = String(e?.message ?? e)
  }
}
import { chatMediaOpen, studioDelete, studioLibrary, studioPowers, voiceStudioSpeak, voiceStudioStatus } from '../api/castle.js'

// VoiceStudio (optional, switched on in Settings) is the Voice power while it is on and running
const voiceUp = ref(false)
const voiceText = ref('')
const voiceBusy = ref(false)
const voiceProblem = ref('')
const voiceClip = ref('')
async function speak() {
  if (voiceBusy.value || !voiceText.value.trim()) return
  voiceBusy.value = true
  voiceProblem.value = ''
  try {
    const path = await voiceStudioSpeak(voiceText.value)
    voiceClip.value = convertFileSrc(path)
    await load()
  } catch (e) {
    voiceProblem.value = String(e)
  } finally {
    voiceBusy.value = false
  }
}

const props = defineProps({
  companion: { type: String, default: 'Your companion' },
  stamp: { type: Number, default: 0 },
})
const emit = defineEmits(['keys'])

const POWERS = [
  { id: 'image', label: 'Pictures', key: 'ChatGPT (OpenAI)', ask: 'Draw me a picture of' },
  { id: 'video', label: 'Videos', key: 'Seedance', ask: 'Make a short video of' },
  { id: 'song', label: 'Songs', key: 'Suno', ask: 'Write and sing a song about' },
  { id: 'voice', label: 'Voice', key: 'ElevenLabs', ask: 'Say this out loud:' },
]
const STATE = {
  ready: { text: 'Ready', cls: 'ready' },
  builtin: { text: 'Codex on your ChatGPT plan', cls: 'ready' },
  prompt: { text: 'Prompt only - add a key', cls: 'prompt' },
}

const powers = ref({})
const lib = ref({ images: [], videos: [], songs: [], voices: [], saved: [], backups: [] })
const tab = ref('images')
const viewerStart = ref(null)
const src = (path) => convertFileSrc(path)

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
const counts = computed(() => ({
  images: lib.value.images.length,
  videos: lib.value.videos.length,
  songs: lib.value.songs.length,
  voices: lib.value.voices.length,
  saved: lib.value.saved.length,
  backups: lib.value.backups.length,
}))

function when(at) {
  return at ? new Date(at * 1000).toLocaleString('en-US', { dateStyle: 'medium', timeStyle: 'short' }) : ''
}

async function load() {
  try {
    voiceUp.value = await voiceStudioStatus().catch(() => false)
    powers.value = await studioPowers()
    lib.value = await studioLibrary()
  } catch {
    // outside Castle's window
  }
}

onMounted(load)
watch(() => props.stamp, load)
</script>

<template>
  <div class="studio">
    <section class="powers">
      <article v-for="p in POWERS" :key="p.id" class="power widget">
        <h3>{{ p.label }}</h3>
        <span v-if="p.id === 'voice' && voiceUp" class="state ready">Ready — VoiceStudio (on this computer)</span>
        <span v-else class="state" :class="STATE[powers[p.id]]?.cls">{{ p.id === 'image' && powers.image === 'prompt' ? 'Prompt only - add an OpenAI key or sign in to Codex' : p.id === 'image' && powers.image === 'ready' ? 'Ready - drawn with your OpenAI key' : STATE[powers[p.id]]?.text ?? '…' }}</span>
        <p class="hint">Ask {{ companion }}: <em>"{{ p.ask }} …"</em></p>
        <template v-if="p.id === 'image'">
          <textarea v-model="imageBrief" rows="2" placeholder="Describe the picture…" aria-label="Picture request" @keydown.ctrl.enter="drawImage"></textarea>
          <button :disabled="!imageBrief.trim()" @click="drawImage">Create picture</button>
          <p v-if="imageProblem" role="alert" class="problem">{{ imageProblem }}</p>
        </template>
        <template v-else-if="p.id === 'voice' && voiceUp">
          <textarea v-model="voiceText" rows="2" placeholder="Type what VoiceStudio should say…" @keydown.ctrl.enter="speak"></textarea>
          <button :disabled="voiceBusy || !voiceText.trim()" @click="speak">{{ voiceBusy ? 'Speaking…' : 'Speak with VoiceStudio' }}</button>
          <ThemedAudio v-if="voiceClip" :src="voiceClip" />
          <p v-if="voiceProblem" role="alert" class="problem">{{ voiceProblem }}</p>
        </template>
        <button v-else-if="powers[p.id] === 'prompt'" @click="emit('keys')">Add the {{ p.key }} key</button>
      </article>
    </section>

    <section class="gallery widget">
      <nav class="tabs">
        <button v-for="t in ['images', 'videos', 'songs', 'voices', 'saved', 'backups']" :key="t" :class="{ active: tab === t }" @click="tab = t; asking = ''">
          {{ { images: 'Pictures', videos: 'Videos', songs: 'Songs', voices: 'Voice', saved: 'Saved prompts', backups: 'Backups' }[t] }} ({{ counts[t] }})
        </button>
        <span class="spacer"></span>
        <ImageJobs @finished="load" />
        <button @click="load">Refresh</button>
      </nav>

      <p v-if="deleteProblem" role="alert" class="problem">{{ deleteProblem }}</p>
      <p v-if="tab === 'backups'" class="hint">Castle keeps the picture it replaces when it draws or you upload a new one. Delete the ones you no longer need; they go to the Recycle Bin.</p>
      <div v-if="tab === 'images' || tab === 'backups'" class="grid">
        <div v-for="(m, i) in shelfPictures" :key="m.path" class="tile">
          <button class="thumb" :title="m.name" @click="viewerStart = i">
            <img :src="src(m.path)" :alt="m.name" loading="lazy" />
          </button>
          <div v-if="asking === m.path" class="confirm" role="group" aria-label="Delete this picture">
            <span>Delete?</span>
            <button class="danger" @click="remove(m.path)">Yes</button>
            <button @click="asking = ''">No</button>
          </div>
          <button v-else class="delete" :title="`Delete ${m.name}`" :aria-label="`Delete ${m.name}`" @click="asking = m.path">✕</button>
        </div>
      </div>
      <div v-else-if="tab === 'videos'" class="list">
        <div v-for="m in lib.videos" :key="m.path" class="video">
          <video :src="src(m.path)" controls preload="metadata"></video>
          <span>{{ m.name }} · {{ when(m.at) }}</span>
          <span v-if="asking === m.path" class="inline-confirm">Delete? <button class="danger" @click="remove(m.path)">Yes</button> <button @click="asking = ''">No</button></span>
          <button v-else class="text-delete" @click="asking = m.path">Delete</button>
        </div>
      </div>
      <div v-else-if="tab === 'songs' || tab === 'voices'" class="list">
        <div v-for="m in lib[tab]" :key="m.path" class="audio">
          <div><strong>{{ m.name }}</strong><span v-if="m.group" class="group">{{ m.group }}</span></div>
          <audio :src="src(m.path)" controls preload="none"></audio>
          <span v-if="asking === m.path" class="inline-confirm">Delete? <button class="danger" @click="remove(m.path)">Yes</button> <button @click="asking = ''">No</button></span>
          <button v-else class="text-delete" @click="asking = m.path">Delete</button>
        </div>
      </div>
      <div v-else class="list">
        <p class="hint">These were saved instead of made, because a key was missing. Add the key and ask {{ companion }} again to make the real thing.</p>
        <div v-for="m in lib.saved" :key="m.path" class="saved">
          <span>{{ m.name }}</span><span class="group">{{ when(m.at) }}</span>
          <button @click="chatMediaOpen(m.path, true)">Show file</button>
          <span v-if="asking === m.path" class="inline-confirm">Delete? <button class="danger" @click="remove(m.path)">Yes</button> <button @click="asking = ''">No</button></span>
          <button v-else class="text-delete" @click="asking = m.path">Delete</button>
        </div>
      </div>
      <p v-if="!counts[tab]" class="empty">Nothing here yet.</p>
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
  backdrop-filter: blur(var(--glass-blur));
}
.power textarea {
  box-sizing: border-box;
  width: 100%;
  min-height: 44px;
  resize: vertical;
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.25);
  color: var(--text);
  font: inherit;
  font-size: 13px;
}
.problem {
  margin: 0;
  font-size: 13px;
  color: var(--warn);
}
.powers {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 14px;
}
.power {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 14px 16px;
}
h3 {
  margin: 0;
  font-size: 15px;
  font-weight: 500;
}
.state {
  align-self: flex-start;
  padding: 3px 10px;
  border-radius: 999px;
  font-size: 12px;
}
.state.ready {
  background: color-mix(in srgb, var(--good) 22%, transparent);
  color: var(--good);
}
.state.prompt {
  background: color-mix(in srgb, var(--warn) 22%, transparent);
  color: var(--warn);
}
.hint,
.empty {
  margin: 0;
  font-size: 13px;
  color: var(--text-dim);
}
.gallery {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
  padding: 14px 18px;
}
.tabs {
  display: flex;
  gap: 6px;
  margin-bottom: 12px;
}
.tabs .active {
  border-color: var(--accent);
  background: var(--hover);
}
.spacer {
  flex: 1;
}
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
  gap: 10px;
  overflow-y: auto;
}
.tile {
  position: relative;
}
.tile .thumb {
  width: 100%;
}
.delete {
  position: absolute;
  top: 6px;
  right: 6px;
  width: 26px;
  height: 26px;
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
  inset: auto 6px 6px 6px;
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
.inline-confirm {
  font-size: 13px;
}
.text-delete {
  justify-self: start;
}
.thumb {
  padding: 0;
  overflow: hidden;
  aspect-ratio: 1;
  border-radius: 10px;
}
.thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.list {
  display: flex;
  flex-direction: column;
  gap: 10px;
  overflow-y: auto;
}
.video {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 13px;
}
.video video {
  max-width: 560px;
  border-radius: 10px;
}
.audio,
.saved {
  display: flex;
  gap: 12px;
  align-items: center;
  justify-content: space-between;
  font-size: 14px;
}
.group {
  margin-left: 10px;
  font-size: 12px;
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
</style>
