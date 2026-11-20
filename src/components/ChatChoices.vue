<script setup>
import { computed } from 'vue'
import { hidesTell, isOwnAnswer, tellOf } from './choices.js'

// The card for a choice Claude Code asks in the Log (a permission prompt, a question, a plan, the
// folder-trust question): the form's tabs, the whole question and its context, each option with its
// description, the Submit of a several-answer question, and Open Log. Only shows; ChatView answers.
const props = defineProps({
  // { question, context?, tabs?, options, submit? } from readChoices
  menu: { type: Object, required: true },
  companion: { type: String, default: 'Your companion' },
})
const emit = defineEmits(['choose', 'submit', 'show-log'])

const tellOption = computed(() => tellOf(props.menu))
// A question whose options carry descriptions shows them, one option per row.
const described = computed(() => props.menu.options.some((option) => option.hint))
const asking = computed(() => props.menu.question || `${props.companion} is asking`)
</script>

<template>
  <div class="choices" role="group" :aria-label="asking">
    <p v-if="menu.tabs" class="tabs">
      <span v-for="tab in menu.tabs" :key="tab.name" :class="{ done: tab.done }">{{ tab.done ? '✓ ' : '' }}{{ tab.name }}</span>
    </p>
    <p class="ask">{{ asking }}</p>
    <p v-if="menu.context" class="context">{{ menu.context }}</p>
    <div class="options" :class="{ described }">
      <template v-for="option in menu.options" :key="option.number">
        <button v-if="!hidesTell(option)" class="option" @click="emit('choose', option.number)">
          <span class="number">{{ option.number }}</span>
          <span class="words">
            <span>{{ option.label }}</span>
            <small v-if="described && option.hint" class="hint">{{ option.hint }}</small>
          </span>
        </button>
      </template>
      <!-- a question of several answers: the boxes are ticked above, this sends them -->
      <button v-if="menu.submit" class="option submit" @click="emit('submit')">{{ menu.submit.label }}</button>
      <button class="option log" @click="emit('show-log')">Open Log</button>
    </div>
    <p v-if="tellOption" class="tell-hint">{{ isOwnAnswer(tellOption) ? 'or type your own answer in the box below' : `or tell ${companion} what to change, in the box below` }}</p>
  </div>
</template>

<style scoped>
.choices {
  display: flex;
  flex-direction: column;
  gap: 6px;
  /* a long form scrolls inside the card, never pushing the box off the screen */
  max-height: 55%;
  overflow-y: auto;
  flex: none;
  margin-bottom: 8px;
  padding: 10px 12px;
  border: 1px solid var(--accent);
  border-radius: 12px;
  background: color-mix(in srgb, var(--accent) 12%, transparent);
}

.ask {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
}

.tabs {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 10px;
  margin: 0;
  font-size: 11px;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--text-dim);
}

.tabs .done {
  color: var(--accent);
}

.context {
  margin: 0;
  font-size: 12px;
  line-height: 1.45;
  color: var(--text-dim);
}

.options {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.options.described {
  flex-direction: column;
  flex-wrap: nowrap;
}

.option {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  max-width: 100%;
  padding: 6px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.2);
  color: var(--text);
  font-size: 12.5px;
  text-align: left;
}

.options.described .option {
  display: flex;
  align-items: flex-start;
  width: 100%;
}

.options.described .option .number {
  margin-top: 1px;
}

.option:hover {
  background: var(--hover);
}

.option .number {
  display: grid;
  place-items: center;
  flex: none;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: var(--accent);
  color: #160f33;
  font-size: 11px;
  font-weight: 700;
}

.option .words {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.option .hint {
  font-size: 11.5px;
  line-height: 1.4;
  color: var(--text-dim);
}

.option.submit {
  border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 30%, transparent);
  font-weight: 600;
}

.option.log {
  color: var(--text-dim);
}

.tell-hint {
  margin: 0;
  font-size: 12px;
  opacity: 0.75;
}

button {
  font: inherit;
  cursor: pointer;
}

button:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
</style>
