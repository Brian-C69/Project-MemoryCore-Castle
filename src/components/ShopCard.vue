<script setup>
// One shop in the town (design marketplace-town-2): the keeper's round portrait, the shop's name on a
// gold ribbon, the seller, three small pictures of what it sells, and Open.
defineProps({ card: { type: Object, required: true } })
const emit = defineEmits(['open'])
</script>

<template>
  <article class="shop-card">
    <div class="portrait">
      <img v-if="card.keeper?.picture" :src="card.keeper.picture" :alt="card.keeper?.name ?? card.name" loading="lazy" />
      <span v-else aria-hidden="true">✦</span>
    </div>
    <div class="ribbon">{{ card.name }}</div>
    <p class="seller">{{ card.seller?.name }}</p>
    <div class="thumbs">
      <img v-for="s in (card.samples ?? []).slice(0, 3)" :key="s" :src="s" alt="" loading="lazy" />
    </div>
    <button class="open" @click="emit('open', card.slug)">✦ Open</button>
  </article>
</template>

<style scoped>
.shop-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 16px 14px 14px;
  border: 1px solid color-mix(in srgb, #e8c26a 45%, var(--line));
  border-radius: 14px;
  background: color-mix(in srgb, var(--panel-background) 80%, transparent);
}
.portrait {
  display: grid;
  place-items: center;
  width: 108px;
  height: 108px;
  overflow: hidden;
  border: 2px solid #e8c26a;
  border-radius: 50%;
  background: color-mix(in srgb, var(--accent) 20%, transparent);
  color: #e8c26a;
  font-size: 30px;
}
.portrait img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.ribbon {
  margin-top: -18px;
  padding: 5px 22px;
  border-radius: 6px;
  background: linear-gradient(180deg, #f3d98f, #d9ae54);
  color: #3a2508;
  font-weight: 600;
  font-size: 15px;
  text-align: center;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.35);
}
.seller {
  margin: 0;
  font-size: 13px;
  color: var(--text-dim);
}
.thumbs {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
  width: 100%;
}
.thumbs img {
  width: 100%;
  aspect-ratio: 1;
  object-fit: cover;
  border: 1px solid var(--line);
  border-radius: 8px;
}
.open {
  padding: 5px 20px;
  border: 1px solid color-mix(in srgb, #e8c26a 55%, var(--line));
  border-radius: 999px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 13px;
  cursor: pointer;
}
.open:hover {
  background: var(--hover);
}
</style>
