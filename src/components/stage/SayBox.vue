<script setup>
// The member's own words for the story: "what do you do?" - shared by every look.
import { ref } from 'vue'

const props = defineProps({ placeholder: { type: String, default: 'Say or do something…' } })
const emit = defineEmits(['say'])
const text = ref('')
const input = ref(null)

function send() {
  const t = text.value.trim()
  if (!t) return
  emit('say', t)
  text.value = ''
}

defineExpose({ focus: () => input.value?.focus() })
</script>

<template>
  <form class="say-box" @submit.prevent="send" @click.stop>
    <input ref="input" v-model="text" :placeholder="props.placeholder" maxlength="2000" />
    <button :disabled="!text.trim()" aria-label="Send">➤</button>
  </form>
</template>

<style scoped>
.say-box {
  display: flex;
  gap: 6px;
}
.say-box input {
  flex: 1;
  min-width: 0;
  padding: 7px 10px;
  border: 1px solid rgba(170, 180, 255, 0.3);
  border-radius: 9px;
  background: rgba(4, 8, 26, 0.6);
  color: #eceaff;
  font: inherit;
  font-size: 13px;
}
.say-box button {
  padding: 0 12px;
  border: 1px solid rgba(170, 180, 255, 0.35);
  border-radius: 9px;
  background: rgba(120, 140, 255, 0.25);
  color: #eceaff;
  cursor: pointer;
}
.say-box button:disabled {
  opacity: 0.4;
}
</style>
