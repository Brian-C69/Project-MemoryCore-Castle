<script setup>
// STUDIO > Marketplace: the town of shops (design marketplace-town-2). Shops | Items and a search;
// a shop opens into its scene (ShopScene: the keeper, her counter, the four steps), and an item opens
// its seller's shop at the explanation. Then My purchases (Install, votes) and My listings (Connect
// Stripe). Everything goes through the core with the member's sign-in. Until the Portfolio's shops
// door answers, the town shows Items only.
import { computed, onMounted, ref } from 'vue'
import { shopConnect, shopInstall, shopListings, shopMine, shopPurchases, shopVote, shopWithdraw, townShop, townShops } from '../api/castle.js'
import ShopCard from './ShopCard.vue'
import ShopScene from './ShopScene.vue'
import { STATUS_WORDS, installState, installedListings, priceText, shopOf, votesText } from './shop.js'

const props = defineProps({
  status: { type: Object, required: true },
  entries: { type: Array, default: () => [] },
})
const emit = defineEmits(['installed'])

const view = ref('town') // town | scene | bought | mine
const mode = ref('shops') // shops | items
const query = ref('')
const cards = ref([])
const shops = ref([])
const shopsMissing = ref(false)
const scene = ref(null) // { detail, start }
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
const search = () => act('search', async () => {
  const [found, items] = await Promise.all([townShops(query.value, 0), shopListings(query.value, 0)])
  shopsMissing.value = !!found?.missing
  shops.value = found?.items ?? []
  cards.value = items?.items ?? []
  if (shopsMissing.value) mode.value = 'items'
})
// a shop from the town opens at its greeting
const enterShop = (slug) => act('open', async () => {
  scene.value = { detail: await townShop(slug), start: null }
  view.value = 'scene'
})
// an item opens its seller's shop at the explanation, with that item in the keeper's hands
const enterWithItem = (card) => act('open', async () => {
  const detail = card.shop?.slug && !shopsMissing.value ? await townShop(card.shop.slug).catch(() => null) : null
  scene.value = { detail, start: card }
  view.value = 'scene'
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
      <button :class="{ active: view === 'town' || view === 'scene' }" @click="view = 'town'">Town</button>
      <button :class="{ active: view === 'bought' }" @click="loadBought">My purchases</button>
      <button :class="{ active: view === 'mine' }" @click="loadMine">My listings</button>
      <span class="spacer"></span>
      <span v-if="busy" class="hint">Working…</span>
    </nav>
    <p v-if="problem" role="alert" class="problem">{{ problem }}</p>
    <p v-else-if="line" class="hint">{{ line }}</p>

    <template v-if="view === 'town'">
      <div class="town-bar">
        <div v-if="!shopsMissing" class="modes" role="radiogroup" aria-label="Shops or items">
          <button role="radio" :aria-checked="mode === 'shops'" :class="{ on: mode === 'shops' }" @click="mode = 'shops'">Shops</button>
          <button role="radio" :aria-checked="mode === 'items'" :class="{ on: mode === 'items' }" @click="mode = 'items'">Items</button>
        </div>
        <form class="search" @submit.prevent="search">
          <input v-model="query" type="search" maxlength="80" placeholder="Search the town" aria-label="Search the town" />
          <button type="submit">Search</button>
        </form>
      </div>
      <div v-if="mode === 'shops' && !shopsMissing" class="town-grid">
        <ShopCard v-for="c in shops" :key="c.slug" :card="c" @open="enterShop" />
        <p v-if="!shops.length && !busy" class="empty">No shops are open yet.</p>
      </div>
      <div v-else class="grid">
        <button v-for="c in cards" :key="c.id" class="card" @click="enterWithItem(c)">
          <img v-if="c.sample" :src="c.sample" :alt="c.name" loading="lazy" />
          <strong>{{ c.name }}</strong>
          <span class="hint">{{ c.shop?.name ?? c.seller?.name }} · {{ priceText(c.priceMyr) }}</span>
          <span class="hint">{{ votesText(c.votes) }}</span>
        </button>
        <p v-if="!cards.length && !busy" class="empty">No workflows for sale yet.</p>
      </div>
    </template>

    <ShopScene v-else-if="view === 'scene' && scene" :key="(scene.detail?.slug ?? '') + (scene.start?.id ?? '')" class="scene-fill" :detail="scene.detail" :start="scene.start" @leave="view = 'town'" />

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
.town-bar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 12px;
}
.modes {
  display: flex;
  overflow: hidden;
  border: 1px solid color-mix(in srgb, #e8c26a 45%, var(--line));
  border-radius: 999px;
}
.modes button {
  border: 0;
  border-radius: 0;
  padding: 6px 22px;
}
.modes .on {
  background: color-mix(in srgb, var(--accent) 70%, transparent);
  color: #07142e;
  font-weight: 600;
}
.town-bar .search {
  flex: 1;
}
.town-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
  gap: 16px;
}
.scene-fill {
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
