<script setup>
import { computed, ref } from 'vue'
import { clipboardCopy } from '../../api/castle.js'
import { devOpenLink } from '../../api/devgit.js'
import ModalSheet from '../../components/ModalSheet.vue'
import { ago, badges } from './git.js'

// One commit, opened from the graph: who and when, the whole message, the files with their line
// counts (each opens its diff), the badges, the short hash to copy, and Open on GitHub when the
// project's origin is on GitHub.
const props = defineProps({
  detail: { type: Object, required: true },
  // 'https://github.com/<owner>/<repo>/commit/' or null
  remote: { type: String, default: null },
})
const emit = defineEmits(['close', 'diff'])

const commit = computed(() => props.detail.commit)
const when = computed(() => {
  const date = new Date(commit.value.date)
  if (Number.isNaN(date.getTime())) return ''
  const long = date.toLocaleString('en-US', { month: 'long', day: 'numeric', year: 'numeric', hour: 'numeric', minute: '2-digit' })
  return `${ago(commit.value.date)} (${long})`
})

const copied = ref(false)
const problem = ref('')

async function copyHash() {
  try {
    await clipboardCopy(commit.value.hash)
    copied.value = true
    setTimeout(() => (copied.value = false), 1500)
  } catch (reason) {
    problem.value = String(reason)
  }
}

async function openOnGitHub() {
  try {
    await devOpenLink(props.remote + commit.value.hash)
  } catch (reason) {
    problem.value = String(reason)
  }
}
</script>

<template>
  <ModalSheet tag="COMMIT" :title="commit.subject" @close="emit('close')">
    <div class="card">
      <p class="who"><strong>{{ commit.author }}</strong><template v-if="when"> &middot; {{ when }}</template></p>
      <pre class="message">{{ detail.message }}</pre>
      <p class="summary">{{ detail.summary }}</p>
      <ul class="files">
        <li v-for="file in detail.files" :key="file.path">
          <button class="link path" :title="`Show the diff of ${file.path}`" @click="emit('diff', file.path)">{{ file.path }}</button>
          <span class="counts">
            <template v-if="file.added === null">binary</template>
            <template v-else>
              <span class="add">+{{ file.added }}</span> <span class="del">-{{ file.removed }}</span>
            </template>
          </span>
        </li>
      </ul>
      <p class="foot">
        <span v-for="badge in badges(commit.refs)" :key="badge.text" class="badge" :class="badge.kind">{{ badge.text }}</span>
        <span class="hash">{{ commit.short }}</span>
        <button class="link" @click="copyHash">{{ copied ? 'copied' : 'copy' }}</button>
        <button v-if="remote" class="link" @click="openOnGitHub">Open on GitHub &#8599;</button>
      </p>
      <p v-if="problem" class="problem" role="alert">{{ problem }}</p>
    </div>
  </ModalSheet>
</template>

<style scoped>
.card {
  font-size: 13.5px;
}

.who {
  margin: 0 0 12px;
}

.message {
  margin: 0 0 14px;
  font-family: inherit;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  line-height: 1.5;
}

.summary {
  margin: 0 0 6px;
  color: var(--text-dim);
}

.files {
  margin: 0 0 14px;
  padding: 0;
  list-style: none;
  font-family: 'Cascadia Mono', Consolas, monospace;
  font-size: 12px;
}

.files li {
  display: flex;
  justify-content: space-between;
  gap: 10px;
  padding: 2px 0;
}

.path {
  min-width: 0;
  overflow-wrap: anywhere;
}

.counts {
  flex: none;
  color: var(--text-dim);
}

.link {
  padding: 0;
  border: 0;
  background: none;
  color: var(--accent);
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.link:hover {
  text-decoration: underline;
}

.link:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}

.add {
  color: var(--good);
}

.del {
  color: var(--warn);
}

.foot {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  margin: 0;
}

.badge {
  padding: 1px 8px;
  border: 1px solid var(--line);
  border-radius: 999px;
  font-size: 12px;
  background: var(--hover);
}

.badge.branch {
  border-color: var(--accent);
  color: var(--accent);
}

.badge.tag {
  border-color: var(--good);
  color: var(--good);
}

.hash {
  font-family: 'Cascadia Mono', Consolas, monospace;
}

.problem {
  margin: 10px 0 0;
  color: var(--warn);
}
</style>
