<script setup>
// Inside a shop (designs marketplace-shop-flow step 1 to 4): the backdrop, the keeper behind the
// counter, and the RPG dialogue box with her name. Greet (her owner's written line), Items (on the
// counter with their prices), Explain (the item held up, what it asks for, her words, the samples),
// Question (ask her anything). Her words come from the visitor's own companion, from public facts only.
import { computed, ref, watch } from 'vue'
import { keeperSay, shopCheckout, shopListing } from '../api/castle.js'
import { asksFor, choicesFor, greetingOf, itemFacts, shopFacts, shopOf, startStep } from './shopscene.js'
import { priceText } from './shop.js'

const props = defineProps({
  /** the shop's detail from the shops door, or null (then the shop is just the item's seller) */
  detail: { type: Object, default: null },
  /** an item picked in the town: the visit starts at its explanation */
  start: { type: Object, default: null },
  /** My shop's preview: the greeting only, nothing is asked or bought */
  preview: { type: Boolean, default: false },
})
const emit = defineEmits(['leave', 'bought'])

const shop = computed(() => shopOf(props.detail, props.start))
const step = ref(startStep(props.start))
const item = ref(null)
const said = ref('')
const asking = ref(false)
const question = ref('')
const asked = ref('')
const problem = ref('')
const line = ref('')

const dialogue = computed(() => {
  if (step.value === 'greet') return greetingOf(shop.value)
  if (step.value === 'items') return shop.value.items.length ? 'Here is everything on my counter today. Which one catches your eye?' : 'My counter is empty today. Come back soon!'
  if (asking.value) return '…'
  if (step.value === 'question' && !said.value) return 'Ask me anything about it.'
  return said.value || item.value?.description || ''
})
const choices = computed(() => (props.preview ? [] : choicesFor(step.value, item.value)))
const asks = computed(() => asksFor(item.value?.blocks))

async function speak(explain) {
  problem.value = ''
  asking.value = true
  try {
    said.value = await keeperSay(explain, shopFacts(shop.value), itemFacts(item.value), explain ? null : question.value)
    if (!explain) asked.value = question.value
    question.value = ''
  } catch (e) {
    problem.value = String(e?.message ?? e)
    if (explain) said.value = item.value?.description ?? ''
  } finally {
    asking.value = false
  }
}

async function pick(card) {
  problem.value = ''
  said.value = ''
  asked.value = ''
  try {
    item.value = { ...card, ...(await shopListing(String(card.id))) }
  } catch {
    item.value = card
  }
  step.value = 'explain'
  speak(true)
}

async function choose(id) {
  if (id === 'leave') return emit('leave')
  if (id === 'items') {
    step.value = 'items'
    said.value = ''
    return
  }
  if (id === 'question') {
    step.value = 'question'
    said.value = ''
    return
  }
  if (id === 'buy' && item.value) {
    try {
      await shopCheckout(String(item.value.id))
      line.value = 'Checkout opened in your browser. After paying, Install it from My purchases.'
      emit('bought')
    } catch (e) {
      problem.value = String(e?.message ?? e)
    }
  }
}

function send() {
  if (!question.value.trim() || asking.value || !item.value) return
  speak(false)
}

// a visit that starts with a picked item explains it at once
watch(
  () => props.start,
  (card) => {
    if (card && !props.preview) pick(card)
  },
  { immediate: true },
)
</script>

<template>
  <section class="scene" :style="shop.backdrop ? { backgroundImage: `url(${shop.backdrop})` } : null" :aria-label="shop.name">
    <button v-if="!preview" class="back" @click="emit('leave')">✦ Back to the town</button>

    <div class="stage">
      <div class="keeper">
        <img v-if="shop.keeperPicture" :src="shop.keeperPicture" :alt="shop.keeperName" />
        <div v-else class="keeper-blank" aria-hidden="true">✦</div>
      </div>

      <div v-if="step === 'items'" class="counter-items">
        <button v-for="c in shop.items" :key="c.id" class="frame" @click="pick(c)">
          <img v-if="c.sample" :src="c.sample" :alt="c.name" />
          <span class="tag">{{ c.name }}<br />{{ priceText(c.priceMyr) }}</span>
        </button>
      </div>

      <div v-else-if="(step === 'explain' || step === 'question') && item" class="held">
        <img v-if="item.samples?.[0] || item.sample" :src="item.samples?.[0] ?? item.sample" :alt="item.name" class="held-picture" />
        <div v-if="asks.length" class="asks">
          <strong>It asks for</strong>
          <span v-for="a in asks" :key="a">✦ {{ a }}</span>
        </div>
      </div>
    </div>

    <div class="dialogue">
      <span class="name-tag">✦ {{ shop.keeperName }}</span>
      <p v-if="asked && step === 'question'" class="asked">“{{ asked }}”</p>
      <p class="line">{{ dialogue }}</p>
      <div v-if="step === 'explain' && (item?.samples?.length ?? 0) > 1" class="samples">
        <img v-for="s in item.samples.slice(0, 3)" :key="s" :src="s" :alt="item.name" />
      </div>
      <p v-if="problem" role="alert" class="problem">{{ problem }}</p>
      <p v-else-if="line" class="hint">{{ line }}</p>
      <form v-if="step === 'question' && !preview" class="ask" @submit.prevent="send">
        <input v-model="question" maxlength="300" :placeholder="`Ask ${shop.keeperName} something`" :aria-label="`Ask ${shop.keeperName} something`" />
        <button type="submit" :disabled="!question.trim() || asking">➤</button>
      </form>
      <div v-if="choices.length" class="choices">
        <button v-for="c in choices" :key="c.id" :class="{ buy: c.id === 'buy' }" @click="choose(c.id)">✦ {{ c.label }}</button>
      </div>
    </div>
  </section>
</template>

<style scoped>
.scene {
  position: relative;
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
  gap: 12px;
  min-height: 420px;
  height: 100%;
  padding: 16px;
  overflow: hidden;
  border: 1px solid color-mix(in srgb, #e8c26a 40%, var(--line));
  border-radius: 14px;
  background: radial-gradient(ellipse at 50% 30%, color-mix(in srgb, var(--accent) 25%, transparent), transparent 70%), var(--panel-background);
  background-size: cover;
  background-position: center;
}
.back {
  position: absolute;
  top: 12px;
  left: 12px;
  z-index: 2;
}
.stage {
  position: relative;
  display: flex;
  flex: 1;
  align-items: flex-end;
  justify-content: center;
  gap: 24px;
  min-height: 0;
}
.keeper {
  height: 100%;
  max-height: 380px;
  display: flex;
  align-items: flex-end;
}
.keeper img {
  max-height: 100%;
  max-width: 340px;
  object-fit: contain;
  border-radius: 12px;
}
.keeper-blank {
  display: grid;
  place-items: center;
  width: 180px;
  height: 220px;
  border-radius: 12px;
  background: color-mix(in srgb, var(--accent) 18%, transparent);
  color: #e8c26a;
  font-size: 42px;
}
.counter-items {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
  align-self: flex-end;
}
.frame {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  width: 130px;
  padding: 6px;
  border: 2px solid #c9a24f;
  border-radius: 8px;
  background: color-mix(in srgb, var(--panel-background) 85%, transparent);
  color: var(--text);
  font: inherit;
  cursor: pointer;
}
.frame:hover {
  box-shadow: 0 0 14px color-mix(in srgb, #e8c26a 70%, transparent);
}
.frame img {
  width: 100%;
  aspect-ratio: 4 / 3;
  object-fit: cover;
  border-radius: 4px;
}
.tag {
  font-size: 12px;
  text-align: center;
}
.held {
  display: flex;
  align-items: flex-end;
  gap: 14px;
}
.held-picture {
  width: 220px;
  border: 3px solid #c9a24f;
  border-radius: 8px;
  box-shadow: 0 6px 22px rgba(0, 0, 0, 0.45);
}
.asks {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 10px 12px;
  border: 1px solid color-mix(in srgb, #e8c26a 45%, var(--line));
  border-radius: 10px;
  background: color-mix(in srgb, var(--panel-background) 88%, transparent);
  font-size: 13px;
}
.dialogue {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 22px 20px 14px;
  border: 1px solid color-mix(in srgb, #e8c26a 50%, var(--line));
  border-radius: 14px;
  background: color-mix(in srgb, var(--panel-background) 92%, transparent);
  -webkit-backdrop-filter: blur(var(--glass-blur)); backdrop-filter: blur(var(--glass-blur));
}
.name-tag {
  position: absolute;
  top: -14px;
  left: 18px;
  padding: 3px 16px;
  border: 1px solid color-mix(in srgb, #e8c26a 55%, var(--line));
  border-radius: 999px;
  background: var(--panel-background);
  font-size: 15px;
}
.line {
  margin: 0;
  font-size: 15px;
  line-height: 1.5;
  white-space: pre-wrap;
}
.asked {
  margin: 0;
  font-size: 13px;
  color: var(--text-dim);
}
.samples {
  display: flex;
  gap: 8px;
}
.samples img {
  width: 90px;
  aspect-ratio: 4 / 3;
  object-fit: cover;
  border: 1px solid var(--line);
  border-radius: 6px;
}
.ask {
  display: flex;
  gap: 8px;
}
.ask input {
  flex: 1;
  padding: 8px 12px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: rgba(0, 0, 0, 0.25);
  color: var(--text);
  font: inherit;
}
.choices {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}
.choices button {
  flex: 1;
  min-width: 150px;
  padding: 8px 14px;
  border-radius: 999px;
}
.choices .buy {
  border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 30%, transparent);
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
  background: color-mix(in srgb, var(--panel-background) 80%, transparent);
  color: var(--text);
  font: inherit;
  font-size: 13px;
  cursor: pointer;
}
button:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
</style>
