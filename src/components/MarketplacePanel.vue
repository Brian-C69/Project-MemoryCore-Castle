<script setup>
// STUDIO > Marketplace: workflows members sell to each other. Browse and search the cards, open one,
// Buy (Stripe Checkout in the browser), then Install it onto the rail; vote on what you bought; and
// My listings with Connect Stripe. Everything goes through the core with the member's sign-in.
import { computed, onMounted, ref } from 'vue'
import { shopCheckout, shopConnect, shopInstall, shopListing, shopListings, shopMine, shopPurchases, shopVote, shopWithdraw } from '../api/castle.js'
import { STATUS_WORDS, installState, installedListings, priceText, shopOf, votesText } from './shop.js'

const props = defineProps({
  status: { type: Object, required: true },
  entries: { type: Array, default: () => [] },
})
const emit = defineEmits(['installed'])

const view = ref('shop') // shop | listing | bought | mine
const query = ref('')
const cards = ref([])
const listing = ref(null)
const purchases = ref([])
const mine = ref([])
const busy = ref('')
const problem = ref('')
const line = ref('')
const installed = computed(() => installedListings(props.entries))
const shop = computed(() => shopOf(props.status))

async function act(what, work) {
  problem.value = ''
  line.value = ''
  busy.value = what
  try {
    await work()
  } catch (e) {
    problem.value = String(e?.message ?? e)
  } finally {
    busy.value = ''
  }
}
const search = () => act('search', async () => (cards.value = (await shopListings(query.value, 0))?.items ?? []))
const open = (id) => act('open', async () => {
  listing.value = await shopListing(id)
  view.value = 'listing'
})
const buy = (id) => act('buy', async () => {
  await shopCheckout(id)
  line.value = 'Checkout opened in your browser. After paying, Install it from My purchases.'
})
const loadBought = () => act('bought', async () => {
  purchases.value = (await shopPurchases()) ?? []
  view.value = 'bought'
})
const install = (p) => act('install', async () => {
  await shopInstall(String(p.id), String(p.listingId))
  line.value = `${p.name} is on your Workflows rail.`
  emit('installed')
})
const vote = (p, up) => act('vote', async () => {
  const next = p.vote === (up ? 'up' : 'down') ? null : up
  await shopVote(String(p.listingId), next)
  p.vote = next === null ? null : next ? 'up' : 'down'
})
const loadMine = () => act('mine', async () => {
  mine.value = (await shopMine()) ?? []
  view.value = 'mine'
})
const connect = () => act('connect', async () => {
  await shopConnect()
  line.value = 'Stripe opened in your browser. Come back here when it is finished.'
})
const withdraw = (l) => act('withdraw', async () => {
  await shopWithdraw(String(l.id))
  await loadMine()
})

onMounted(search)
</script>

<template>
  <section class="shop widget" aria-label="Marketplace">
    <nav class="tabs">
      <button :class="{ active: view === 'shop' || view === 'listing' }" @click="view = 'shop'">Shop</button>
      <button :class="{ active: view === 'bought' }" @click="loadBought">My purchases</button>
      <button :class="{ active: view === 'mine' }" @click="loadMine">My listings</button>
      <span class="spacer"></span>
      <span v-if="busy" class="hint">Working…</span>
    </nav>
    <p v-if="problem" role="alert" class="problem">{{ problem }}</p>
    <p v-else-if="line" class="hint">{{ line }}</p>

    <template v-if="view === 'shop'">
      <form class="search" @submit.prevent="search">
        <input v-model="query" type="search" maxlength="80" placeholder="Search workflows" aria-label="Search workflows" />
        <button type="submit">Search</button>
      </form>
      <div class="grid">
        <button v-for="c in cards" :key="c.id" class="card" @click="open(c.id)">
          <img v-if="c.sample" :src="c.sample" :alt="c.name" loading="lazy" />
          <strong>{{ c.name }}</strong>
          <span class="hint">{{ c.seller?.name }} · {{ priceText(c.priceMyr) }}</span>
          <span class="hint">{{ votesText(c.votes) }}</span>
        </button>
        <p v-if="!cards.length && !busy" class="empty">No workflows for sale yet.</p>
      </div>
    </template>

    <div v-else-if="view === 'listing' && listing" class="listing">
      <button class="back" @click="view = 'shop'">← Back to the shop</button>
      <h4>{{ listing.name }}</h4>
      <p class="hint">{{ listing.seller?.name }} · {{ priceText(listing.priceMyr) }} · {{ votesText(listing.votes) }}</p>
      <p>{{ listing.description }}</p>
      <div class="samples">
        <img v-for="s in listing.samples ?? []" :key="s" :src="s" :alt="listing.name" />
      </div>
      <p v-if="listing.blocks?.length" class="hint">It asks for: {{ listing.blocks.map((b) => b.label || b.type).join(', ') }}</p>
      <button v-if="listing.mine" disabled>This is yours</button>
      <button v-else-if="listing.bought || installed.has(String(listing.id))" @click="loadBought">You have it - see My purchases</button>
      <button v-else class="primary" :disabled="!!busy" @click="buy(listing.id)">Buy for {{ priceText(listing.priceMyr) }}</button>
    </div>

    <div v-else-if="view === 'bought'" class="rows">
      <div v-for="p in purchases" :key="p.id" class="row">
        <strong>{{ p.name }}</strong>
        <span class="spacer"></span>
        <button :class="{ on: p.vote === 'up' }" aria-label="Thumbs up" @click="vote(p, true)">👍</button>
        <button :class="{ on: p.vote === 'down' }" aria-label="Thumbs down" @click="vote(p, false)">👎</button>
        <button v-if="installState(p, installed) === 'install'" class="primary" @click="install(p)">Install</button>
        <button v-else @click="install(p)">Update</button>
      </div>
      <p v-if="!purchases.length" class="empty">Nothing bought yet.</p>
    </div>

    <div v-else-if="view === 'mine'" class="rows">
      <div class="row">
        <span>{{ shop.connected ? 'Stripe is connected: buyers pay you directly.' : 'Connect Stripe to be paid for your workflows.' }}</span>
        <span class="spacer"></span>
        <button v-if="!shop.connected" class="primary" @click="connect">Connect Stripe</button>
      </div>
      <div v-for="l in mine" :key="l.id" class="row">
        <strong>{{ l.name }}</strong>
        <span class="tag">{{ STATUS_WORDS[l.status] ?? l.status }}</span>
        <span class="hint">{{ priceText(l.priceMyr) }} · {{ votesText(l.votes) }} · {{ l.sales ?? 0 }} sold · {{ priceText(l.earnedMyr ?? 0) }} earned</span>
        <span class="spacer"></span>
        <button @click="withdraw(l)">Withdraw</button>
        <p v-if="l.note" class="note">{{ l.note }}</p>
      </div>
      <p v-if="!mine.length" class="empty">{{ shop.selling ? 'Publish one of your workflows from its card.' : 'Selling opens soon.' }}</p>
    </div>
  </section>
</template>

<style scoped>
.shop {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
  padding: 14px 18px;
  overflow-y: auto;
  scrollbar-width: var(--scrollbar-width);
  scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track);
}
.tabs,
.search,
.row {
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
.search input {
  flex: 1;
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.25);
  color: var(--text);
  font: inherit;
}
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 12px;
}
.card {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 10px;
  border-radius: 12px;
  text-align: left;
}
.card img {
  width: 100%;
  aspect-ratio: 3 / 2;
  object-fit: cover;
  border-radius: 8px;
}
.listing {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.listing h4 {
  margin: 0;
  font-size: 18px;
}
.samples {
  display: flex;
  gap: 10px;
  overflow-x: auto;
}
.samples img {
  height: 200px;
  border-radius: 10px;
}
.back {
  align-self: flex-start;
}
.rows {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.row {
  padding: 10px 14px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: color-mix(in srgb, var(--panel-background) 70%, transparent);
}
.row .on {
  border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 25%, transparent);
}
.tag {
  padding: 1px 8px;
  border: 1px solid var(--line);
  border-radius: 999px;
  font-size: 11px;
}
.note {
  width: 100%;
  margin: 0;
  font-size: 13px;
  color: var(--warn);
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
