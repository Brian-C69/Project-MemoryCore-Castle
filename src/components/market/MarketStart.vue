<script setup>
// Before the first account (page 2): one card that sets the paper money.
import { ref } from 'vue'
import { QUICK_MONEY, parsePaperMoney } from './market.js'

defineProps({ busy: { type: String, default: '' } })
const emit = defineEmits(['start'])

const amount = ref('10,000')
const problem = ref('')

function start() {
  const money = parsePaperMoney(amount.value)
  if (money === null) {
    problem.value = 'Type an amount above 0 and up to 1,000,000,000.'
    return
  }
  problem.value = ''
  emit('start', money)
}
</script>

<template>
  <div class="start">
    <section class="card">
      <h3>SET YOUR PAPER MONEY</h3>
      <p>Practice with pretend money on real prices. Nothing real is ever ordered.</p>
      <label class="amount">
        <input v-model="amount" inputmode="decimal" aria-label="Paper money in USD" @keydown.enter="start" />
        <span>USD</span>
      </label>
      <div class="quick">
        <button v-for="q in QUICK_MONEY" :key="q" :class="{ on: amount === q.toLocaleString('en-US') }" @click="amount = q.toLocaleString('en-US')">{{ q.toLocaleString('en-US') }}</button>
      </div>
      <p v-if="problem" class="problem">{{ problem }}</p>
      <button class="go" :disabled="!!busy" @click="start">{{ busy || 'Start' }}</button>
    </section>
  </div>
</template>

<style scoped>
.start {
  display: grid;
  place-items: center;
  height: 100%;
}

.card {
  display: flex;
  flex-direction: column;
  gap: 14px;
  width: 440px;
  max-width: 100%;
  padding: 28px 32px;
  border: 1px solid var(--line);
  border-radius: 16px;
  background: var(--panel-background);
  backdrop-filter: blur(var(--glass-blur));
  text-align: center;
}

h3 {
  margin: 0;
  font-size: 13px;
  letter-spacing: 0.2em;
}

p {
  margin: 0;
  color: var(--text-dim);
  font-size: 13px;
}

.amount {
  display: flex;
  align-items: center;
  padding: 10px 14px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: rgba(0, 0, 0, 0.2);
}

.amount input {
  flex: 1;
  border: 0;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 26px;
  outline: none;
}

.amount span {
  color: var(--text-dim);
}

.quick {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
}

.quick button {
  padding: 7px 0;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: none;
  color: var(--text);
  font: inherit;
  cursor: pointer;
}

.quick button.on {
  background: color-mix(in srgb, var(--accent) 35%, transparent);
}

.problem {
  color: var(--warn);
}

.go {
  padding: 11px 0;
  border: 0;
  border-radius: 12px;
  background: var(--accent);
  color: var(--bg, #0b1020);
  font: inherit;
  font-weight: 600;
  letter-spacing: 0.14em;
  cursor: pointer;
}
</style>
