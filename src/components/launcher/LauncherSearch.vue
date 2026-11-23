<script setup>
// The search box at the top of APPS. Typing filters the rooms; the room handles Enter, the arrow keys and Esc.
import { ref } from 'vue'

defineProps({ modelValue: { type: String, default: '' } })
defineEmits(['update:modelValue'])
const input = ref(null)
defineExpose({ focus: () => input.value?.focus() })
</script>

<template>
  <div class="search">
    <label class="sr" for="launcher-search">Search rooms</label>
    <input id="launcher-search" ref="input" type="text" role="searchbox" :value="modelValue" placeholder="Search rooms" autocomplete="off" spellcheck="false" maxlength="60" @input="$emit('update:modelValue', $event.target.value)" />
  </div>
</template>

<style scoped>
.search { flex: 1; max-width: 420px; margin: 0 auto; }
.sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); }
input {
  width: 100%; box-sizing: border-box; padding: 9px 14px; color: var(--text); font: inherit; font-size: 14px;
  background: var(--panel-background); border: 1px solid var(--line); border-radius: 14px;
}
input::placeholder { color: var(--text-dim); }
/* one ring, not two: the border itself turns to the accent when the box has focus */
input:focus, input:focus-visible { outline: none; border-color: var(--accent); box-shadow: 0 0 0 1px var(--accent); }
</style>
