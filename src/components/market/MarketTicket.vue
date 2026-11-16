<script setup>
// The order ticket (design 1): Buy or Sell, Market or Limit, quantity with quick shares, TP and SL,
// leverage and margin mode, reduce-only, a live cost preview, and the refusal said plainly.
import { computed, ref, watch } from 'vue'
import { clientId, qtyForShare, usd } from './market.js'

const props = defineProps({
  symbol: { type: String, required: true },
  quote: { type: Object, default: null },
  instrument: { type: Object, default: null },
  available: { type: Number, default: 0 },
  busy: { type: String, default: '' },
  preview: { type: Function, required: true },
})
const emit = defineEmits(['place', 'leverage'])

const side = ref('LONG')
const orderType = ref('MARKET')
const qty = ref('')
const limitPrice = ref('')
const tp = ref('')
const sl = ref('')
const reduceOnly = ref(false)
const cost = ref(null)
const leverage = ref(10)
const marginMode = ref('CROSS')

watch(
  () => props.instrument,
  (i) => {
    if (!i) return
    leverage.value = i.leverage
    marginMode.value = i.marginMode
  },
  { immediate: true },
)

const maxLeverage = computed(() => props.instrument?.maxLeverage ?? 100)
const priceNow = computed(() => (orderType.value === 'LIMIT' && Number(limitPrice.value) > 0 ? Number(limitPrice.value) : props.quote?.last ?? 0))

function share(part) {
  qty.value = String(qtyForShare(props.available, part, priceNow.value, leverage.value, props.instrument?.qtyStep ?? 0.001))
}

const num = (v) => (String(v).trim() === '' ? null : Number(v))

function order() {
  return {
    symbol: props.symbol,
    side: side.value,
    orderType: orderType.value,
    qty: Number(qty.value),
    price: orderType.value === 'LIMIT' ? num(limitPrice.value) : null,
    tp: num(tp.value),
    sl: num(sl.value),
    reduceOnly: reduceOnly.value,
    clientOrderId: clientId(),
  }
}

let wait
watch([side, orderType, qty, limitPrice, tp, sl, reduceOnly, () => props.symbol], () => {
  clearTimeout(wait)
  cost.value = null
  if (!(Number(qty.value) > 0)) return
  wait = setTimeout(async () => {
    try {
      cost.value = await props.preview(order())
    } catch (reason) {
      cost.value = { refusal: String(reason?.message ?? reason) }
    }
  }, 300)
})

function submit() {
  if (!(Number(qty.value) > 0)) return
  emit('place', order())
}

function saveLeverage() {
  emit('leverage', marginMode.value, Number(leverage.value))
}
</script>

<template>
  <section class="panel ticket">
    <h3>ORDER</h3>
    <div class="sides">
      <button class="buy" :class="{ on: side === 'LONG' }" @click="side = 'LONG'">Buy</button>
      <button class="sell" :class="{ on: side === 'SHORT' }" @click="side = 'SHORT'">Sell</button>
    </div>
    <div class="types" role="group" aria-label="Order type">
      <button :class="{ on: orderType === 'MARKET' }" @click="orderType = 'MARKET'">Market</button>
      <button :class="{ on: orderType === 'LIMIT' }" @click="orderType = 'LIMIT'">Limit</button>
    </div>
    <label v-if="orderType === 'LIMIT'" class="field">PRICE<input v-model="limitPrice" inputmode="decimal" placeholder="Limit price" /></label>
    <label class="field">QUANTITY<input v-model="qty" inputmode="decimal" :placeholder="symbol.replace('USDT', '')" /></label>
    <div class="shares">
      <button v-for="p in [0.25, 0.5, 0.75, 1]" :key="p" @click="share(p)">{{ p * 100 }}%</button>
    </div>
    <div class="exits">
      <label class="field">TP<input v-model="tp" inputmode="decimal" placeholder="—" /></label>
      <label class="field">SL<input v-model="sl" inputmode="decimal" placeholder="—" /></label>
    </div>
    <div class="lev">
      <span>Leverage {{ leverage }}x</span>
      <input v-model.number="leverage" type="range" min="1" :max="maxLeverage" step="1" @change="saveLeverage" />
      <div class="modes">
        <button :class="{ on: marginMode === 'CROSS' }" @click="marginMode = 'CROSS'; saveLeverage()">Cross</button>
        <button :class="{ on: marginMode === 'ISOLATED' }" @click="marginMode = 'ISOLATED'; saveLeverage()">Isolated</button>
      </div>
    </div>
    <label class="toggle"><input v-model="reduceOnly" type="checkbox" /> Reduce only</label>
    <p v-if="cost?.refusal" class="refusal">{{ cost.refusal }}</p>
    <p v-else-if="cost" class="cost">Cost {{ usd(cost.cost) }} · fee {{ usd(cost.fee) }}</p>
    <button class="place" :disabled="!!busy || !(Number(qty) > 0) || !!cost?.refusal" @click="submit">{{ busy || 'Place order' }}</button>
  </section>
</template>

<style scoped>
.ticket {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 14px 16px;
}

h3 {
  margin: 0;
  font-size: 11px;
  letter-spacing: 0.16em;
  color: var(--text-dim);
}

.sides {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}

.sides button {
  padding: 10px 0;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: none;
  color: var(--text);
  font: inherit;
  font-weight: 600;
  cursor: pointer;
}

.sides .buy.on {
  background: color-mix(in srgb, var(--good) 55%, transparent);
  border-color: var(--good);
}

.sides .sell.on {
  background: color-mix(in srgb, var(--bad) 55%, transparent);
  border-color: var(--bad);
}

.types,
.modes,
.shares {
  display: grid;
  grid-auto-flow: column;
  gap: 6px;
}

.types button,
.modes button,
.shares button {
  padding: 5px 0;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: none;
  color: var(--text-dim);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}

.types button.on,
.modes button.on {
  background: color-mix(in srgb, var(--accent) 30%, transparent);
  color: var(--text);
}

.field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 10px;
  letter-spacing: 0.14em;
  color: var(--text-dim);
}

.field input {
  padding: 7px 9px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.2);
  color: var(--text);
  font: inherit;
  font-size: 13px;
  letter-spacing: normal;
}

.exits {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}

.lev {
  display: flex;
  flex-direction: column;
  gap: 6px;
  font-size: 12px;
}

.toggle {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-dim);
}

.cost,
.refusal {
  margin: 0;
  font-size: 12px;
  color: var(--text-dim);
}

.refusal {
  color: var(--warn);
}

.place {
  padding: 10px 0;
  border: 0;
  border-radius: 10px;
  background: var(--accent);
  color: var(--bg, #0b1020);
  font: inherit;
  font-weight: 600;
  cursor: pointer;
}

.place:disabled {
  opacity: 0.55;
  cursor: default;
}
</style>
