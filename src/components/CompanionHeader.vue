<script setup>
// The top of the companion panel: the portrait (their picture, or the glowing crystal), the
// name, and which app they think with - as in the Constellation design.
defineProps({
  name: { type: String, default: 'Your companion' },
  cliLabel: { type: String, default: 'Claude Code' },
  avatar: { type: String, default: null },
})
</script>

<template>
  <div class="companion-header">
    <div class="avatar">
      <img v-if="avatar" :src="avatar" alt="" />
      <span v-else class="crystal">◆</span>
    </div>
    <div class="who">
      <p class="name">{{ name }}</p>
      <span class="chip">{{ cliLabel }}</span>
    </div>
  </div>
</template>

<style scoped>
.companion-header {
  display: flex;
  gap: 18px;
  align-items: center;
  padding: 4px 6px 6px;
}
.avatar {
  display: grid;
  place-items: center;
  flex: none;
  width: 84px;
  height: 84px;
  overflow: hidden;
  border: 2px solid var(--accent);
  border-radius: 50%;
  background: radial-gradient(circle at 40% 35%, #2a55b8, #0a1638 70%);
  box-shadow: 0 0 22px rgba(120, 165, 255, 0.35);
}
.avatar img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.crystal {
  font-size: 38px;
  color: #cfe0ff;
  text-shadow: 0 0 16px #7aa6ff, 0 0 4px #ffffff;
}
.name {
  margin: 0 0 8px;
  font-size: 22px;
  font-weight: 400;
}
.chip {
  padding: 4px 12px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: var(--hover);
  font-size: 13px;
}
</style>
