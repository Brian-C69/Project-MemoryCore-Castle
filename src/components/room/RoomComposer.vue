<script setup>
// The chat room's composer: the "Replying to" strip (with its cancel) and the message box.
// v-model is the draft; `send` and `cancel` go up to the room.
defineProps({
  modelValue: { type: String, default: '' },
  // "Replying to Mira: the start of her line", or '' when not replying
  replying: { type: String, default: '' },
  disabled: { type: Boolean, default: false },
})
const emit = defineEmits(['update:modelValue', 'send', 'cancel'])
</script>

<template>
  <p v-if="replying" class="replying">
    <span>{{ replying }}</span>
    <button type="button" aria-label="Stop replying" @click="emit('cancel')">&#x2715;</button>
  </p>
  <form class="say" @submit.prevent="emit('send')">
    <input :value="modelValue" maxlength="1000" placeholder="Write to the room..." :disabled="disabled" @input="emit('update:modelValue', $event.target.value)" />
    <button class="primary" :disabled="!modelValue.trim()">Send</button>
  </form>
</template>

<style scoped>
.replying {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0 0 6px;
  padding: 6px 10px;
  border-left: 3px solid var(--accent);
  border-radius: 6px;
  background: color-mix(in srgb, var(--accent) 12%, transparent);
  font-size: 13px;
}
.replying span {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.replying button {
  padding: 0 6px;
  border: 0;
  background: none;
  color: var(--text);
  cursor: pointer;
}
.say {
  display: flex;
  gap: 8px;
  margin-top: 10px;
}
.say input {
  flex: 1;
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.25);
  color: var(--text);
  font: inherit;
  font-size: 14px;
}
.say button {
  padding: 7px 14px;
  border: 1px solid var(--accent);
  border-radius: 8px;
  background: var(--accent);
  color: #07142e;
  font: inherit;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
}
.say button:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
</style>
