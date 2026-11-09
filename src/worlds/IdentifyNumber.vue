<script setup>
// The big number shown on one screen for a moment when the member presses Identify.
defineProps({ number: { type: Number, required: true } })
</script>

<template>
  <div class="identify">
    <span class="number">{{ number }}</span>
  </div>
</template>

<style scoped>
.identify {
  position: fixed;
  inset: 0;
  display: grid;
  place-items: center;
  border: 6px solid #8f7bff;
  box-sizing: border-box;
  animation: fade 2.2s ease forwards;
}
.number {
  font: 700 22vh/1 system-ui, sans-serif;
  color: #ffffff;
  text-shadow: 0 0 24px #8f7bff, 0 0 60px #6b5cff;
}
@keyframes fade {
  0%, 75% { opacity: 1; }
  100% { opacity: 0; }
}
</style>
