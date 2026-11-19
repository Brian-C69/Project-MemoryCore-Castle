<script setup>
// My shop (the Keep's alice-shop-setup-1): the shop's name, the keeper's name and picture, the
// greeting, the keeper card, the backdrop and Open on the left; What visitors see on the right (the
// shop scene in preview, at the greeting). The keeper card is reviewed by Kiyo before visitors see it.
import { computed, onMounted, ref } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import { myShop, myShopSave, workflowPickPicture } from '../api/castle.js'
import ShopScene from './ShopScene.vue'
import { CARD_WORDS, shopForm, shopFormProblem } from './shop.js'

const props = defineProps({ companion: { type: String, default: 'Your companion' } })

const mine = ref(null)
const form = ref(shopForm(null, props.companion))
const keeperPicture = ref('') // a newly picked file
const backdrop = ref('')
const busy = ref(false)
const problem = ref('')
const done = ref('')

const preview = computed(() => ({
  slug: 'preview',
  name: form.value.name || 'Your shop',
  keeper: { name: form.value.keeperName, picture: keeperPicture.value ? convertFileSrc(keeperPicture.value) : mine.value?.keeperPicture ?? '' },
  backdrop: backdrop.value ? convertFileSrc(backdrop.value) : mine.value?.backdrop ?? '',
  greeting: form.value.greeting,
  keeperCard: form.value.keeperCard,
  seller: { name: '' },
  items: [],
}))

async function load() {
  try {
    mine.value = await myShop()
    form.value = shopForm(mine.value, props.companion)
  } catch (e) {
    problem.value = String(e?.message ?? e)
  }
}
async function pick(which) {
  problem.value = ''
  try {
    const path = await workflowPickPicture(which === 'keeper' ? 'The keeper’s picture' : 'The shop’s backdrop')
    if (path) (which === 'keeper' ? keeperPicture : backdrop).value = path
  } catch (e) {
    problem.value = String(e?.message ?? e)
  }
}
async function save() {
  done.value = ''
  problem.value = shopFormProblem(form.value, keeperPicture.value, backdrop.value)
  if (problem.value) return
  busy.value = true
  try {
    mine.value = await myShopSave(form.value, keeperPicture.value || null, backdrop.value || null)
    keeperPicture.value = ''
    backdrop.value = ''
    done.value = 'Saved. A changed keeper card waits for review; the approved one stays shown.'
  } catch (e) {
    problem.value = String(e?.message ?? e)
  } finally {
    busy.value = false
  }
}
onMounted(load)
</script>

<template>
  <div class="setup">
    <section class="form widget">
      <h3>✦ {{ form.name || 'My shop' }}</h3>
      <label class="row"><span>Shop name</span><input v-model="form.name" maxlength="60" placeholder="Mira's Atelier" /></label>
      <label class="row"><span>Keeper name</span><input v-model="form.keeperName" maxlength="40" /></label>
      <div class="row">
        <span>Keeper picture</span>
        <div class="pick">
          <img v-if="preview.keeper.picture" :src="preview.keeper.picture" alt="The keeper" class="round" />
          <button @click="pick('keeper')">{{ preview.keeper.picture ? 'Change' : 'Choose' }}</button>
        </div>
      </div>
      <label class="row"><span>Greeting</span><input v-model="form.greeting" maxlength="160" placeholder="Welcome, traveller. Take your time." /></label>
      <label class="row top">
        <span>Keeper card</span>
        <span class="card-box">
          <textarea v-model="form.keeperCard" rows="5" maxlength="1200" placeholder="Tell your visitors about your shop, your story, or what makes it special…"></textarea>
          <small>{{ form.keeperCard.length }} / 1200</small>
        </span>
      </label>
      <p v-if="mine?.cardReview" class="hint">Keeper card: {{ CARD_WORDS[mine.cardReview] ?? mine.cardReview }}<template v-if="mine.cardNote"> - {{ mine.cardNote }}</template></p>
      <div class="row">
        <span>Backdrop</span>
        <div class="pick">
          <img v-if="preview.backdrop" :src="preview.backdrop" alt="The backdrop" class="wide" />
          <button @click="pick('backdrop')">{{ preview.backdrop ? 'Change' : 'Choose' }}</button>
        </div>
      </div>
      <label class="row"><span>Open</span><input v-model="form.open" type="checkbox" class="switch" /></label>
      <p v-if="problem" role="alert" class="problem">{{ problem }}</p>
      <p v-else-if="done" class="hint">{{ done }}</p>
      <button class="primary save" :disabled="busy" @click="save">{{ busy ? 'Saving…' : '✦ Save' }}</button>
    </section>
    <section class="visitors widget">
      <h3>✦ What visitors see</h3>
      <ShopScene :detail="preview" preview />
    </section>
  </div>
</template>

<style scoped>
.setup {
  display: grid;
  grid-template-columns: minmax(320px, 1fr) minmax(360px, 1.2fr);
  gap: 16px;
  min-height: 0;
}
.form,
.visitors {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
  padding: 16px 18px;
  border: 1px solid color-mix(in srgb, #e8c26a 40%, var(--line));
  border-radius: 14px;
  background: color-mix(in srgb, var(--panel-background) 85%, transparent);
  overflow-y: auto;
}
h3 {
  margin: 0;
  font-size: 18px;
  font-weight: 500;
}
.row {
  display: grid;
  grid-template-columns: 120px 1fr;
  align-items: center;
  gap: 12px;
  font-size: 14px;
}
.row.top {
  align-items: start;
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
.switch {
  width: auto;
}
.card-box {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.card-box small {
  align-self: flex-end;
  color: var(--text-dim);
}
.pick {
  display: flex;
  align-items: center;
  gap: 12px;
}
.round {
  width: 72px;
  height: 72px;
  object-fit: cover;
  border: 2px solid #e8c26a;
  border-radius: 50%;
}
.wide {
  width: 180px;
  aspect-ratio: 16 / 9;
  object-fit: cover;
  border: 1px solid var(--line);
  border-radius: 8px;
}
.save {
  align-self: flex-start;
  padding: 8px 28px;
  border-radius: 999px;
}
.problem {
  margin: 0;
  font-size: 13px;
  color: var(--warn);
}
.hint {
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
