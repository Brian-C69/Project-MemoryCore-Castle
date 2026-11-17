<script setup>
// One outfit or hairstyle in APPEARANCE's closet (the chosen design, appearance-room-2): its
// turnaround big, its name, four small icon buttons and the Wear pill; its words open in place.
import { ref } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'

const props = defineProps({
  item: { type: Object, required: true },
  worn: { type: Boolean, default: false },
  busy: { type: Boolean, default: false },
  // "Draw turnaround · about $0.25" when the key pays, else "Draw turnaround"
  drawLabel: { type: String, default: 'Draw turnaround' },
  price: { type: String, default: '' },
  stamp: { type: [String, Number], default: '' },
  // the picture's small copy (made once by Castle); shown instead of the big picture while it is there
  thumb: { type: String, default: '' },
})
const emit = defineEmits(['wear', 'upload', 'draw', 'save', 'view'])

const picture = () => props.item.sheet || props.item.reference
const src = (path) => props.thumb || convertFileSrc(path) + (props.stamp !== '' ? `?v=${props.stamp}` : '')

const editing = ref(null) // { name, words } while the words are open
function edit() {
  editing.value = { name: props.item.name, words: props.item.note ?? '' }
}
function save() {
  emit('save', { name: editing.value.name, words: editing.value.words })
  editing.value = null
}
</script>

<template>
  <article class="closet-card" :class="{ worn }">
    <button v-if="picture()" class="picture" :title="`View ${item.name}`" @click="emit('view')">
      <img :src="src(picture())" :alt="item.name" />
    </button>
    <div v-else class="picture empty">{{ item.savedPrompt ? 'Prompt saved - add the key to draw' : 'Not drawn yet' }}</div>

    <div class="body">
      <template v-if="editing">
        <input v-model="editing.name" maxlength="80" aria-label="Name" />
        <textarea v-model="editing.words" rows="3" maxlength="4000" aria-label="Words"></textarea>
        <div class="row">
          <button class="pill primary" :disabled="!editing.name.trim() || busy" @click="save">Save</button>
          <button class="pill" :disabled="busy" @click="editing = null">Cancel</button>
        </div>
      </template>
      <template v-else>
        <h4>{{ item.name }}</h4>
        <p v-if="item.note" class="words">{{ item.note }}</p>
      </template>

      <div class="actions">
        <div class="icons">
          <button class="icon" :disabled="busy" :title="drawLabel" :aria-label="drawLabel" @click="emit('draw')">
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3l1.8 4.9L19 9.7l-5.2 1.8L12 16.5l-1.8-5L5 9.7l5.2-1.8z" /><path d="M18.5 15l.8 2.1 2.2.8-2.2.8-.8 2.1-.8-2.1-2.2-.8 2.2-.8z" /></svg>
            <span v-if="price" class="price">{{ price }}</span>
          </button>
          <button class="icon" :disabled="busy" :title="item.reference ? 'Replace reference' : 'Upload reference'" :aria-label="item.reference ? 'Replace reference' : 'Upload reference'" @click="emit('upload')">
            <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="3.5" y="5" width="17" height="14" rx="2" /><circle cx="9" cy="10" r="1.6" /><path d="M5 17l4.5-4.5 3 3 2.5-2.5 4 4" /></svg>
          </button>
          <button class="icon" :disabled="busy || !!editing" title="Edit words" aria-label="Edit words" @click="edit">
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 20l1-4L16 5l3 3L8 19z" /><path d="M14 7l3 3" /></svg>
          </button>
          <button class="icon" :disabled="!picture()" :title="`View ${item.name}`" :aria-label="`View ${item.name}`" @click="emit('view')">
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12z" /><circle cx="12" cy="12" r="3" /></svg>
          </button>
        </div>
        <button class="pill wear" :class="{ primary: !worn, wearing: worn }" :disabled="busy" @click="emit('wear')">{{ worn ? 'Wearing' : 'Wear' }}</button>
      </div>
    </div>
  </article>
</template>

<style scoped>
.closet-card {
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: color-mix(in srgb, var(--panel-background) 70%, transparent);
}
.closet-card.worn {
  border-color: color-mix(in srgb, var(--accent) 70%, var(--line));
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent) 35%, transparent);
}
.picture {
  display: grid;
  place-items: center;
  aspect-ratio: 16 / 9;
  padding: 0;
  border: 0;
  border-bottom: 1px solid var(--line);
  border-radius: 0;
  background: color-mix(in srgb, var(--panel-background) 50%, #0a1638);
  cursor: zoom-in;
}
.picture img {
  width: 100%;
  height: 100%;
  object-fit: contain;
}
.picture.empty {
  color: var(--text-dim);
  font-size: 13px;
  cursor: default;
}
.body {
  display: grid;
  gap: 8px;
  padding: 12px 14px 14px;
}
h4 {
  margin: 0;
  font-size: 14px;
  font-weight: 500;
  letter-spacing: 0.22em;
  text-transform: uppercase;
}
.words {
  margin: 0;
  font-size: 13px;
  color: var(--text-dim);
}
input,
textarea {
  font: inherit;
  color: var(--text);
  background: var(--panel-background);
  border: 1px solid var(--line);
  border-radius: 8px;
  padding: 6px 8px;
  resize: vertical;
}
.row,
.actions,
.icons {
  display: flex;
  align-items: center;
  gap: 8px;
}
.actions {
  justify-content: space-between;
}
.icon {
  position: relative;
  display: grid;
  place-items: center;
  width: 40px;
  height: 34px;
  padding: 0;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: transparent;
  color: var(--text);
  cursor: pointer;
}
.icon svg {
  width: 18px;
  height: 18px;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.5;
  stroke-linejoin: round;
  stroke-linecap: round;
}
.icon:hover:not(:disabled) {
  background: var(--hover);
}
.icon:disabled {
  opacity: 0.45;
  cursor: default;
}
.price {
  position: absolute;
  top: -8px;
  right: -10px;
  padding: 1px 5px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--accent) 85%, #000);
  color: #fff;
  font-size: 10px;
}
.pill {
  padding: 7px 18px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: transparent;
  color: var(--text);
  cursor: pointer;
}
.pill.primary {
  border-color: transparent;
  background: var(--accent);
  color: #0b1530;
}
.pill.wearing {
  border-color: color-mix(in srgb, var(--accent) 70%, var(--line));
  color: var(--accent);
}
.wear {
  min-width: 110px;
}
</style>
