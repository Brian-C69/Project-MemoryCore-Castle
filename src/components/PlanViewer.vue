<script setup>
import { computed } from 'vue'
import { firstHeading } from './markdown.js'
import MarkdownText from './MarkdownText.vue'
import ModalSheet from './ModalSheet.vue'

// A plan (or a Gate story part) read whole, in Castle's themed window.
const props = defineProps({
  // The whole text as the companion wrote it (markdown).
  plan: { type: String, required: true },
  // The label at the top: PLAN, or STORY for a Gate chapter.
  tag: { type: String, default: 'PLAN' },
})
const emit = defineEmits(['close'])

const title = computed(() => firstHeading(props.plan) ?? '')
</script>

<template>
  <ModalSheet :tag="tag" :title="title" @close="emit('close')">
    <MarkdownText :text="plan" size="reader" />
  </ModalSheet>
</template>
