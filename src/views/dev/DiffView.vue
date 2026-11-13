<script setup>
import { computed } from 'vue'
import ModalSheet from '../../components/ModalSheet.vue'
import { diffLines } from './git.js'

// A diff, read only, in Castle's reader window: added lines green, removed lines red, the hunk
// lines lit. Every line is plain text (no HTML is ever made from the diff).
const props = defineProps({
  title: { type: String, default: '' },
  text: { type: String, default: '' },
})
const emit = defineEmits(['close'])
const lines = computed(() => diffLines(props.text))
</script>

<template>
  <ModalSheet tag="DIFF" :title="title" @close="emit('close')">
    <p v-if="!text.trim()" class="quiet">No difference to show.</p>
    <pre v-else class="diff"><span v-for="(line, n) in lines" :key="n" :class="line.kind">{{ line.text }}
</span></pre>
  </ModalSheet>
</template>

<style scoped>
.diff {
  margin: 0;
  font-family: 'Cascadia Mono', Consolas, monospace;
  font-size: 12.5px;
  line-height: 1.45;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

.quiet {
  margin: 0;
  color: var(--text-dim);
}

.add {
  color: var(--good);
  background: color-mix(in srgb, var(--good) 10%, transparent);
}

.del {
  color: var(--warn);
  background: color-mix(in srgb, var(--warn) 10%, transparent);
}

.hunk {
  color: var(--accent);
}

.meta {
  color: var(--text-dim);
}
</style>
