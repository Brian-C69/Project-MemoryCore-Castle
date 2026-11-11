<script setup>
import { ref, onMounted, onBeforeUnmount } from 'vue'
import { imageDismiss, imageJobs, imageRetry } from '../api/castle.js'
const problem = ref('')
const active = (job) => ['queued', 'working', 'switching'].includes(job.phase)
async function dismiss(id = null) { try { await imageDismiss(id); problem.value = ''; await refresh() } catch (e) { problem.value = String(e) } }
async function retry(id) { try { await imageRetry(id); problem.value = ''; await refresh() } catch (e) { problem.value = String(e) } }
const jobs = ref([])
let timer
async function refresh() { try { jobs.value = await imageJobs() } catch {} }
onMounted(() => { refresh(); timer = setInterval(refresh, 1500) })
onBeforeUnmount(() => clearInterval(timer))
</script>
<template>
  <details v-if="jobs.length" class="image-jobs" open>
    <summary>Image requests</summary>
    <p v-for="job in jobs" :key="job.id" role="status"><strong>{{ job.phase }}</strong> — {{ job.message }} <button v-if="job.phase === 'prompt-saved'" @click="retry(job.id)">Retry with Codex</button> <button v-if="!active(job)" title="Take this off the list" @click="dismiss(job.id)">Dismiss</button></p>
    <p v-if="jobs.some((j) => !active(j))"><button @click="dismiss()">Clear finished</button></p>
    <p v-if="problem" role="alert">{{ problem }}</p>
  </details>
</template>
<style scoped>.image-jobs { padding: 10px; border: 1px solid var(--line); border-radius: 8px; font-size: 13px; } p { margin: 6px 0; }</style>
