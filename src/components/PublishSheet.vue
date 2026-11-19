<script setup>
// Publish one of the member's own workflows on the Marketplace: one to three sample pictures made
// with it, a price and a line of words. Kiyo reviews it in the Keep before it goes on sale.
import { ref } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import { shopPublish, workflowPickPicture } from '../api/castle.js'

const props = defineProps({ entry: { type: Object, required: true } })
const emit = defineEmits(['close', 'published'])

const samples = ref([])
const price = ref(5)
const words = ref(props.entry.workflow?.description ?? '')
const busy = ref(false)
const problem = ref('')

async function addSample() {
  problem.value = ''
  try {
    const path = await workflowPickPicture('A picture made with this workflow')
    if (path && samples.value.length < 3) samples.value.push(path)
  } catch (e) {
    problem.value = String(e?.message ?? e)
  }
}
async function publish() {
  problem.value = ''
  busy.value = true
  try {
    await shopPublish(props.entry.id, null, samples.value, Number(price.value), words.value)
    emit('published')
  } catch (e) {
    problem.value = String(e?.message ?? e)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="sheet" role="dialog" aria-modal="true" :aria-label="`Publish ${entry.workflow?.name}`" @keydown.esc="emit('close')">
    <div class="box widget">
      <h3>PUBLISH {{ (entry.workflow?.name ?? '').toUpperCase() }}</h3>
      <div class="samples">
        <img v-for="s in samples" :key="s" :src="convertFileSrc(s)" alt="A sample" />
        <button v-if="samples.length < 3" class="add" @click="addSample">＋ Sample picture</button>
      </div>
      <label><span class="label">Price (RM)</span><input v-model="price" type="number" min="1" step="0.5" /></label>
      <label><span class="label">What it makes</span><textarea v-model="words" rows="3" maxlength="300"></textarea></label>
      <p class="hint">Kiyo reviews it before it goes on sale. Buyers pay you through your Stripe; the Marketplace keeps its share.</p>
      <p v-if="problem" role="alert" class="problem">{{ problem }}</p>
      <div class="row">
        <span class="spacer"></span>
        <button @click="emit('close')">Cancel</button>
        <button class="primary" :disabled="busy || !samples.length" @click="publish">{{ busy ? 'Sending…' : 'Send for review' }}</button>
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
.box {
  display: flex;
  flex-direction: column;
  gap: 12px;
  width: min(560px, 100%);
  padding: 18px 20px;
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
.samples {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
}
.samples img,
.add {
  width: 140px;
  height: 100px;
  border-radius: 10px;
  object-fit: cover;
}
.add {
  border-style: dashed;
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
}
.row {
  display: flex;
  gap: 8px;
}
.spacer {
  flex: 1;
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
button:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
</style>
