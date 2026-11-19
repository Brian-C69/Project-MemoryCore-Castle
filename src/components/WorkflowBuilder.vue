<script setup>
// Make your own workflow: its name, icon and words, a shelf of blocks to add, each block's own
// settings, and the picture step's prompt with tap-to-insert names. Save keeps the member's private
// copy (a new one, or theirs when editing); the core's checks show beside the block they belong to.
import { computed, ref, watch } from 'vue'
import { workflowCheck, workflowSave } from '../api/castle.js'
import GlassSelect from './GlassSelect.vue'
import HelpTip from './HelpTip.vue'
import { ICONS, QUALITIES, SHELF, SIZES, addBlock, draftOf, insertable, moveBlock, removeBlock } from './workflows.js'

const props = defineProps({
  /** the workflow to start from (null: a new one) */
  start: { type: Object, default: null },
  /** one of the member's own ids, to overwrite (Edit); empty makes a new one */
  replace: { type: String, default: '' },
})
const emit = defineEmits(['saved', 'close'])

const draft = ref(draftOf(props.start))
const problems = ref([])
const saving = ref(false)
const saveProblem = ref('')
const create = computed(() => draft.value.blocks.find((b) => b.type === 'create'))
const inputs = computed(() => draft.value.blocks.filter((b) => b.type !== 'create'))
const followable = computed(() => draft.value.blocks.filter((b) => b.type === 'character' || b.type === 'picture'))
const pictures = computed(() => draft.value.blocks.filter((b) => b.type === 'picture'))
const texts = computed(() => draft.value.blocks.filter((b) => b.type === 'text'))
const names = computed(() => insertable(draft.value))
const problemsOf = (id) => problems.value.filter((p) => p.block === id).map((p) => p.message)
const general = computed(() => problems.value.filter((p) => !p.block).map((p) => p.message))
const KIND = Object.fromEntries(SHELF)

// the core's own checks, a moment after each change
let timer
watch(
  draft,
  () => {
    clearTimeout(timer)
    timer = setTimeout(async () => {
      try {
        problems.value = await workflowCheck(draft.value)
      } catch {
        problems.value = []
      }
    }, 300)
  },
  { deep: true, immediate: true },
)

const promptBox = ref(null)
function insert(token) {
  const box = promptBox.value
  const text = create.value.prompt ?? ''
  const at = box ? box.selectionStart : text.length
  create.value.prompt = text.slice(0, at) + token + text.slice(box ? box.selectionEnd : at)
}
function toggleReference(id) {
  const refs = create.value.references ?? (create.value.references = [])
  const i = refs.indexOf(id)
  if (i >= 0) refs.splice(i, 1)
  else refs.push(id)
}
const optionsText = (b) => (b.options ?? []).join(', ')
const setOptions = (b, text) => (b.options = text.split(',').map((o) => o.trim()).filter(Boolean))

async function save() {
  saveProblem.value = ''
  saving.value = true
  try {
    const id = await workflowSave(draft.value, props.replace || null)
    emit('saved', id)
  } catch (e) {
    if (Array.isArray(e)) problems.value = e
    else saveProblem.value = String(e?.message ?? e)
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="sheet" role="dialog" aria-modal="true" aria-label="Make your own workflow" @keydown.esc="emit('close')">
    <div class="builder widget">
      <header>
        <h3 class="with-tip">{{ replace ? 'EDIT WORKFLOW' : 'NEW WORKFLOW' }}<HelpTip text="Saved for you alone. Selling opens when the Marketplace does." /></h3>
        <span class="spacer"></span>
        <button @click="emit('close')">Cancel</button>
        <button class="primary" :disabled="saving || problems.length > 0" @click="save">{{ saving ? 'Saving…' : 'Save' }}</button>
      </header>

      <div class="cols">
        <section class="about">
          <label><span class="label">Name</span><input v-model="draft.name" maxlength="60" placeholder="Two cats in the garden" /></label>
          <div>
            <span class="label">Icon</span>
            <div class="icons" role="radiogroup" aria-label="Icon">
              <button v-for="(glyph, key) in ICONS" :key="key" role="radio" :aria-checked="draft.icon === key" :class="{ on: draft.icon === key }" :title="key" @click="draft.icon = key">{{ glyph }}</button>
            </div>
          </div>
          <label><span class="label">What it makes</span><textarea v-model="draft.description" rows="2" maxlength="300" placeholder="One line for the card"></textarea></label>
          <div>
            <span class="label">Add a block</span>
            <div class="shelf">
              <button v-for="[type, word] in SHELF" :key="type" @click="addBlock(draft, type)">+ {{ word }}</button>
            </div>
          </div>
          <p v-for="m in general" :key="m" class="problem">{{ m }}</p>
          <p v-if="saveProblem" role="alert" class="problem">{{ saveProblem }}</p>
        </section>

        <section class="blocks">
          <div v-for="b in inputs" :key="b.id" class="block">
            <div class="block-head">
              <span class="kind">{{ KIND[b.type] ?? b.type }}</span>
              <span class="spacer"></span>
              <button title="Up" aria-label="Move up" @click="moveBlock(draft, b.id, -1)">↑</button>
              <button title="Down" aria-label="Move down" @click="moveBlock(draft, b.id, 1)">↓</button>
              <button class="danger" title="Remove" aria-label="Remove" @click="removeBlock(draft, b.id)">✕</button>
            </div>
            <label><span class="label">Label</span><input v-model="b.label" maxlength="60" /></label>
            <label v-if="b.type === 'chips'"><span class="label">Choices, separated by commas</span><input :value="optionsText(b)" @change="setOptions(b, $event.target.value)" /></label>
            <div v-if="b.type === 'describe'" class="pair">
              <label><span class="label">Reads</span>
                <GlassSelect v-model="b.from" placeholder="Choose a picture block" :options="pictures.map((p) => ({ value: p.id, label: p.label || p.id }))" aria-label="Reads" />
              </label>
              <label><span class="label">Writes into</span>
                <GlassSelect v-model="b.into" placeholder="Choose a text box" :options="texts.map((t) => ({ value: t.id, label: t.label || t.id }))" aria-label="Writes into" />
              </label>
            </div>
            <p v-for="m in problemsOf(b.id)" :key="m" class="problem">{{ m }}</p>
          </div>

          <div v-if="create" class="block create-block">
            <div class="block-head"><span class="kind">Picture step</span></div>
            <label>
              <span class="label">Prompt</span>
              <textarea ref="promptBox" v-model="create.prompt" rows="5" maxlength="4000"></textarea>
            </label>
            <div v-if="names.length" class="inserts">
              <span class="label">Tap to insert</span>
              <button v-for="n in names" :key="n.token" class="chip" @click="insert(n.token)">{{ n.label }}</button>
            </div>
            <div v-if="followable.length" class="inserts">
              <span class="label">Follow these pictures (at most five)</span>
              <label v-for="f in followable" :key="f.id" class="tick">
                <input type="checkbox" :checked="(create.references ?? []).includes(f.id)" @change="toggleReference(f.id)" /> {{ f.label || f.id }}
              </label>
            </div>
            <div class="pair">
              <label><span class="label">Shape</span>
                <GlassSelect v-model="create.size" :options="SIZES.map(([v, w]) => ({ value: v, label: `${w} (${v})` }))" aria-label="Shape" />
              </label>
              <label><span class="label">Quality</span>
                <GlassSelect v-model="create.quality" :options="QUALITIES" aria-label="Quality" />
              </label>
            </div>
            <p v-for="m in problemsOf(create.id)" :key="m" class="problem">{{ m }}</p>
          </div>
        </section>
      </div>
    </div>
  </div>
</template>

<style scoped>
.sheet {
  position: absolute;
  inset: 0;
  z-index: 20;
  display: grid;
  place-items: center;
  padding: 24px;
  background: rgba(0, 0, 0, 0.45);
}
.builder {
  display: flex;
  flex-direction: column;
  gap: 14px;
  width: min(1100px, 100%);
  max-height: 100%;
  padding: 18px 20px;
  background: var(--panel-background);
  border: 1px solid var(--line);
  border-radius: 14px;
  -webkit-backdrop-filter: blur(var(--glass-blur)); backdrop-filter: blur(var(--glass-blur));
}
header {
  display: flex;
  align-items: center;
  gap: 10px;
}
h3 {
  margin: 0;
  font-size: 13px;
  font-weight: 500;
  letter-spacing: 0.18em;
}
h3.with-tip {
  display: flex;
  align-items: center;
}
.spacer {
  flex: 1;
}
.cols {
  display: grid;
  grid-template-columns: minmax(260px, 340px) 1fr;
  gap: 18px;
  min-height: 0;
}
.about,
.blocks {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
  overflow-y: auto;
  scrollbar-width: var(--scrollbar-width);
  scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track);
}
.label {
  display: block;
  margin-bottom: 4px;
  font-size: 12px;
  color: var(--text-dim);
}
input,
textarea {
  width: 100%;
  box-sizing: border-box;
  padding: 7px 10px;
  border: 1px solid var(--line);
  border-radius: 9px;
  background: rgba(0, 0, 0, 0.25);
  color: var(--text);
  font: inherit;
  font-size: 14px;
}
.tick input {
  width: auto;
}
textarea {
  resize: vertical;
}
.icons,
.shelf,
.inserts {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
}
.inserts .label {
  width: 100%;
}
.icons button {
  width: 36px;
  height: 36px;
  padding: 0;
  font-size: 16px;
}
.icons .on {
  border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 25%, transparent);
}
.block {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: color-mix(in srgb, var(--panel-background) 70%, transparent);
}
.create-block {
  border-color: color-mix(in srgb, var(--accent) 55%, var(--line));
}
.block-head {
  display: flex;
  align-items: center;
  gap: 6px;
}
.kind {
  font-size: 12px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--accent);
}
.pair {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}
.chip {
  border-radius: 999px;
}
.tick {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
}
.problem {
  margin: 0;
  font-size: 13px;
  color: var(--warn);
}
.hint {
  margin: 0;
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
button.primary {
  background: var(--accent);
  border-color: var(--accent);
  color: #07142e;
  font-weight: 600;
}
.danger {
  border-color: color-mix(in srgb, #ff6b81 60%, var(--line));
  color: #ff9aa9;
}
button:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
</style>
