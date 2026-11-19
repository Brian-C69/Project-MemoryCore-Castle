<script setup>
import { computed, ref, watch } from 'vue'
import { projectShelve, projectState } from '../api/castle.js'
import { MOVES, kidsQuestion, stateLine } from './projectshelf.js'

// Above an open project in the PROJECTS room: where it is (active, paused, archived or recently
// deleted) and the moves from there. The companion moves projects the same way with
// `castle-memory project pause | resume | archive | restore | delete`. A parent with sub-projects
// on its shelf asks whether they move with it (`--with-subprojects` for the companion).
const props = defineProps({
  file: { type: String, required: true }, // db/project/<id>.md
  companion: { type: String, default: '' },
  // changes when the project may have changed elsewhere (a task ticked, a sub-project moved)
  stamp: { type: Number, default: 0 },
})
const emit = defineEmits(['moved'])

const state = ref(null)
const problem = ref('')
const asking = ref('') // 'pause' while the reason is asked, 'delete' while the yes is asked, 'kids' for the sub-projects
const pending = ref('') // the shelf a 'kids' question is about
const note = ref('')
const busy = ref(false)

const moves = computed(() => (state.value ? MOVES[state.value.shelf] ?? [] : []))
// the sub-projects question for the move being asked about (pause, delete or a plain one)
const kidsAsk = computed(() => kidsQuestion(state.value, asking.value === 'pause' ? 'paused' : asking.value === 'delete' ? 'deleted' : pending.value))

async function load() {
  problem.value = ''
  asking.value = ''
  try {
    state.value = await projectState(props.file)
  } catch (reason) {
    state.value = null
    problem.value = String(reason?.message ?? reason)
  }
}

async function move(to, withKids = false) {
  busy.value = true
  problem.value = ''
  try {
    await projectShelve(props.file, to, to === 'paused' ? note.value : null, withKids)
    asking.value = ''
    pending.value = ''
    note.value = ''
    await load()
    emit('moved')
  } catch (reason) {
    problem.value = String(reason?.message ?? reason)
  }
  busy.value = false
}

function choose(m) {
  if (m.ask) asking.value = m.ask
  else if (kidsQuestion(state.value, m.to)) {
    pending.value = m.to
    asking.value = 'kids'
  } else move(m.to)
}

watch(() => [props.file, props.stamp], load, { immediate: true })
</script>

<template>
  <div v-if="state" class="shelf-bar" :class="state.shelf">
    <div class="where">
      <span class="chip">{{ stateLine(state) }}</span>
      <span v-if="state.note" class="note">“{{ state.note }}”</span>
    </div>
    <div v-if="!asking" class="moves">
      <button v-for="m in moves" :key="m.label" :class="{ danger: m.to === 'deleted' }" :disabled="busy" @click="choose(m)">{{ m.label }}</button>
    </div>

    <form v-if="asking === 'pause'" class="ask" @submit.prevent="move('paused')">
      <label for="pause-note">Why is it paused? <span class="dim">(optional)</span></label>
      <input id="pause-note" v-model="note" maxlength="500" placeholder="Waiting on the client's photos" />
      <p v-if="kidsAsk" class="kids-q">{{ kidsAsk.question }}</p>
      <div v-if="kidsAsk" class="row">
        <button class="primary" type="button" :disabled="busy" @click="move('paused', true)">{{ kidsAsk.all }}</button>
        <button type="submit" :disabled="busy">{{ kidsAsk.one }}</button>
        <button type="button" @click="asking = ''">Cancel</button>
      </div>
      <div v-else class="row">
        <button class="primary" type="submit" :disabled="busy">Pause {{ state.name }}</button>
        <button type="button" @click="asking = ''">Cancel</button>
      </div>
    </form>

    <div v-if="asking === 'kids' && kidsAsk" class="ask" role="alertdialog" aria-labelledby="kids-q">
      <p id="kids-q"><strong>{{ kidsAsk.question }}</strong> They stand on the same shelf as {{ state.name }}.</p>
      <div class="row">
        <button class="primary" :disabled="busy" @click="move(pending, true)">{{ kidsAsk.all }}</button>
        <button :disabled="busy" @click="move(pending)">{{ kidsAsk.one }}</button>
        <button @click="asking = ''">Cancel</button>
      </div>
    </div>

    <div v-if="asking === 'delete'" class="ask danger-box" role="alertdialog" aria-labelledby="delete-q">
      <p id="delete-q"><strong>Delete {{ state.name }}?</strong> It moves to Recently deleted for 30 days, where you can restore it. After that it is gone for good.</p>
      <p v-if="kidsAsk" class="kids-q">{{ kidsAsk.question }}</p>
      <div class="row">
        <template v-if="kidsAsk">
          <button class="danger" :disabled="busy" @click="move('deleted', true)">{{ kidsAsk.all }}</button>
          <button class="danger" :disabled="busy" @click="move('deleted')">{{ kidsAsk.one }}</button>
        </template>
        <button v-else class="danger" :disabled="busy" @click="move('deleted')">Delete {{ state.name }}</button>
        <button @click="asking = ''">Keep it</button>
      </div>
    </div>
    <p v-if="problem" class="problem">{{ problem }}</p>
  </div>
  <p v-else-if="problem" class="problem">{{ problem }}</p>
</template>

<style scoped>
.shelf-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 10px 16px;
  align-items: center;
  justify-content: space-between;
  margin: 0 0 14px;
  padding: 10px 12px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: color-mix(in srgb, var(--hover) 60%, transparent);
}
.shelf-bar.paused {
  border-style: dashed;
}
.where {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: baseline;
  min-width: 0;
}
.chip {
  padding: 2px 9px;
  border-radius: 999px;
  border: 1px solid currentColor;
  font-size: 12px;
  color: var(--good);
  white-space: nowrap;
}
.paused .chip {
  color: var(--warn);
}
.archived .chip {
  color: var(--text-dim);
}
.deleted .chip {
  color: var(--bad);
}
.note {
  font-size: 13px;
  color: var(--text-dim);
  overflow-wrap: anywhere;
}
.moves,
.row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
button {
  padding: 5px 12px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--hover);
  color: var(--text);
  font: inherit;
  font-size: 13px;
  cursor: pointer;
}
button:hover {
  border-color: var(--accent);
}
button.primary {
  background: var(--accent);
  border-color: var(--accent);
  color: var(--background, #050b1f);
  font-weight: 600;
}
button.danger {
  border-color: color-mix(in srgb, var(--bad) 60%, transparent);
  color: var(--bad);
  background: color-mix(in srgb, var(--bad) 10%, transparent);
}
button:disabled {
  opacity: 0.5;
  cursor: default;
}
button:focus-visible,
input:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
.ask {
  display: grid;
  gap: 8px;
  width: 100%;
  font-size: 13px;
}
.ask input {
  padding: 6px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.25);
  color: var(--text);
  font: inherit;
}
.ask p {
  margin: 0;
}
.danger-box {
  padding: 10px 12px;
  border: 1px solid color-mix(in srgb, var(--bad) 45%, transparent);
  border-radius: 10px;
  background: color-mix(in srgb, var(--bad) 7%, transparent);
}
.dim {
  color: var(--text-dim);
}
.kids-q {
  margin: 0;
  font-weight: 600;
}
.problem {
  width: 100%;
  margin: 0;
  font-size: 13px;
  color: var(--warn);
}
</style>
