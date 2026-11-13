<script setup>
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { devPlanBody, devPlanItems } from '../../api/castle.js'
import PlanViewer from '../../components/PlanViewer.vue'
import { groupByPhase, itemMark, openPlans, planProgress } from './devtabs.js'

// A DEV tab's Plan panel: the project's open plans and their boxes, ticking as the work goes on.
// Read only; it reads every 10 seconds while it shows.
const props = defineProps({
  // the DEV tab's slot
  index: { type: Number, required: true },
  // the panel is showing (it reads only then)
  shown: { type: Boolean, default: false },
})

const READ_EVERY_MS = 10000

const answer = ref(null)
const problem = ref('')
const reading = ref(null)
const opening = ref(null)
let timer

const plans = computed(() =>
  openPlans(answer.value?.plans).map((plan) => ({ ...plan, groups: groupByPhase(plan.items), progress: planProgress(plan.items) })),
)

async function refresh() {
  try {
    answer.value = await devPlanItems(props.index)
    problem.value = ''
  } catch (reason) {
    problem.value = String(reason)
  }
}

async function showWhole(plan) {
  opening.value = plan.id
  try {
    reading.value = await devPlanBody(props.index, plan.id)
    problem.value = ''
  } catch (reason) {
    problem.value = String(reason)
  } finally {
    opening.value = null
  }
}

watch(
  () => props.shown,
  (on) => {
    clearInterval(timer)
    if (!on) return
    refresh()
    timer = setInterval(refresh, READ_EVERY_MS)
  },
  { immediate: true },
)
onBeforeUnmount(() => clearInterval(timer))
</script>

<template>
  <aside class="plan-panel" aria-label="Plan">
    <header>
      <span class="tag">PLAN</span>
      <span class="spacer"></span>
      <button class="small" title="Read again" aria-label="Read again" @click="refresh">&#8635;</button>
    </header>

    <div class="scroll">
      <p v-if="problem" class="problem">{{ problem }}</p>
      <p v-else-if="answer && !answer.available" class="quiet">{{ answer.message || 'No plan can be read for this project.' }}</p>
      <p v-else-if="answer && !plans.length" class="quiet">No plan is open in this project.</p>
      <p v-else-if="!answer" class="quiet">Reading the plan…</p>

      <section v-for="plan in plans" :key="plan.id" class="plan">
        <h3 :title="plan.title">{{ plan.title || 'Plan' }}</h3>
        <p class="meta">
          <span v-if="plan.phase">Phase {{ plan.phase }}</span>
          <span v-if="plan.progress">{{ plan.progress }} done</span>
        </p>
        <template v-for="(group, g) in plan.groups" :key="g">
          <h4 v-if="group.heading" :class="{ done: group.state === 'done' }">{{ group.heading }}</h4>
          <ul>
            <li v-for="item in group.items" :key="item.ordinal" :class="[item.kind, item.state]">
              <span class="box" :title="itemMark(item.state).label" :aria-label="itemMark(item.state).label">{{ itemMark(item.state).mark }}</span>
              <span class="text">{{ item.text }}</span>
            </li>
          </ul>
        </template>
        <button class="small whole" :disabled="opening === plan.id" @click="showWhole(plan)">{{ opening === plan.id ? 'Opening…' : 'Show the whole plan' }}</button>
      </section>
    </div>

    <PlanViewer v-if="reading !== null" :plan="reading" @close="reading = null" />
  </aside>
</template>

<style scoped>
.plan-panel {
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  padding: 14px 10px 14px 16px;
  background: var(--panel-background);
  border: 1px solid var(--line);
  border-radius: 14px;
  backdrop-filter: blur(var(--glass-blur));
  box-sizing: border-box;
}

header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 6px 8px 0;
  border-bottom: 1px solid var(--line);
}

.tag {
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.2em;
  color: var(--accent);
}

.spacer {
  flex: 1;
}

.scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 8px 6px 0 0;
  scrollbar-width: thin;
  scrollbar-color: var(--line) transparent;
}

.quiet {
  margin: 8px 0;
  font-size: 13px;
  color: var(--text-dim);
}

.problem {
  margin: 8px 0;
  font-size: 13px;
  color: var(--warn);
}

.plan + .plan {
  margin-top: 16px;
  padding-top: 12px;
  border-top: 1px solid var(--line);
}

h3 {
  margin: 4px 0 2px;
  font-size: 14px;
  font-weight: 600;
  line-height: 1.35;
  overflow-wrap: anywhere;
}

.meta {
  display: flex;
  gap: 10px;
  margin: 0 0 6px;
  font-size: 11px;
  color: var(--text-dim);
}

h4 {
  margin: 10px 0 4px;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-dim);
  overflow-wrap: anywhere;
}

h4.done {
  color: var(--good);
}

ul {
  margin: 0;
  padding: 0;
  list-style: none;
}

li {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 3px 0;
  font-size: 13px;
  line-height: 1.4;
}

.box {
  display: grid;
  place-items: center;
  flex: none;
  width: 16px;
  height: 16px;
  margin-top: 1px;
  border: 1px solid var(--line);
  border-radius: 4px;
  font-size: 11px;
  line-height: 1;
}

.text {
  min-width: 0;
  overflow-wrap: anywhere;
}

li.seal .text {
  color: var(--accent);
}

li.done .box {
  border-color: var(--good);
  color: var(--good);
}

li.done .text {
  color: var(--text-dim);
}

li.carried .box {
  color: var(--text-dim);
}

li.carried .text {
  color: var(--text-dim);
  font-style: italic;
}

li.flagged .box {
  border-color: var(--warn);
  color: var(--warn);
}

.small {
  padding: 4px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}

.small:hover:not(:disabled) {
  background: var(--hover);
}

.small:disabled {
  opacity: 0.5;
  cursor: wait;
}

.small:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}

.whole {
  margin-top: 10px;
}
</style>
