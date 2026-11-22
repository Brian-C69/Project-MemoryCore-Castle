<script setup>
// One platform's card on MESSAGING. The shell (icon, name, status pill, switch, blurb, guide button)
// is shared by every platform; the body is either the token + pending-approval + paired/connected
// shape that Telegram and Discord use, or - for a platform with its own flow (WhatsApp's QR) - the
// default slot. Purely presentational: the room owns the status polling and the save/approve/forget
// calls, this just renders whatever status it's handed and emits plain events back.
import { computed, ref } from 'vue'
import './automations/roomcard.css'

const props = defineProps({
  title: { type: String, required: true },
  blurb: { type: String, default: '' },
  icon: { type: String, default: 'chat' },
  on: { type: Boolean, default: false },
  busy: { type: Boolean, default: false },
  status: { type: Object, default: null },
  tokenPlaceholder: { type: String, default: 'Bot token' },
  // A platform with its own body (the default slot) passes its own pill instead of the token one.
  custom: { type: Boolean, default: false },
  pillText: { type: String, default: '' },
  pillTone: { type: String, default: '' },
})
const emit = defineEmits(['toggle', 'save-token', 'approve', 'forget', 'refresh', 'guide'])

const token = ref('')
function saveToken() {
  const value = token.value.trim()
  if (!value) return
  emit('save-token', value)
  token.value = ''
}

const pill = computed(() => {
  if (props.custom && props.pillText) return { text: props.pillText, tone: props.pillTone || 'idle' }
  if (!props.on) return { text: 'Off', tone: 'off' }
  const s = props.status
  if (!s) return { text: 'Checking…', tone: 'idle' }
  if (!s.hasToken) return { text: 'Needs a token', tone: 'warn' }
  if (s.pending) return { text: 'Waiting for you', tone: 'warn' }
  if (s.paired) return s.connected ? { text: 'Connected', tone: 'ok' } : { text: 'Not connected', tone: 'warn' }
  return s.connected ? { text: 'Ready to pair', tone: 'idle' } : { text: 'Not connected', tone: 'warn' }
})

// Which of the three setup steps is next, for the little progress dots.
const step = computed(() => {
  const s = props.status
  if (!props.on || !s) return 0
  if (!s.hasToken) return 1
  if (s.paired) return 4
  if (s.pending) return 3
  return 2
})
const STEPS = ['Token', 'Message it', 'Approve']
</script>

<template>
  <section class="platform castle-room-card" :class="{ live: on, [`tone-${pill.tone}`]: true }">
    <header class="top">
      <span class="badge" aria-hidden="true">
        <svg v-if="icon === 'telegram'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M21.5 3.5 2.8 10.7c-.8.3-.8 1.4 0 1.7l4.6 1.6 1.8 5.6c.2.7 1.1.9 1.6.4l2.6-2.4 4.7 3.4c.6.4 1.4.1 1.6-.6L22.9 5c.2-.9-.6-1.8-1.4-1.5Z" /><path d="m7.4 14 9.6-7.2" /></svg>
        <svg v-else-if="icon === 'discord'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M7.5 6.5c1.4-.6 2.8-.9 4.5-.9s3.1.3 4.5.9c1.8 2.7 2.7 5.5 2.9 8.4-1.2 1-2.6 1.6-4 2l-.9-1.6c.5-.2 1-.4 1.4-.7-1.2.5-2.5.8-3.9.8s-2.7-.3-3.9-.8c.4.3.9.5 1.4.7l-.9 1.6c-1.4-.4-2.8-1-4-2 .2-2.9 1.1-5.7 2.9-8.4Z" /><circle cx="9.4" cy="12.2" r="1" fill="currentColor" /><circle cx="14.6" cy="12.2" r="1" fill="currentColor" /></svg>
        <svg v-else-if="icon === 'whatsapp'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M3.5 20.5 5 16A8.5 8.5 0 1 1 8 19l-4.5 1.5Z" /><path d="M9 8.8c.3 3 2.6 5.4 5.6 5.9l1.2-1.3-2-1-.8.7a4 4 0 0 1-1.8-1.8l.7-.8-1-2Z" /></svg>
        <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M4 5h16v11H9l-5 4V5Z" /></svg>
      </span>
      <div class="name">
        <h4>{{ title }}</h4>
        <span class="pill" :class="pill.tone"><i></i>{{ pill.text }}</span>
      </div>
      <input
        class="switch"
        type="checkbox"
        role="switch"
        :aria-label="`${title} on or off`"
        :checked="on"
        :disabled="busy"
        @change="emit('toggle', $event.target.checked)"
      />
    </header>

    <p v-if="blurb" class="blurb">{{ blurb }}</p>

    <div v-if="on" class="body">
      <slot v-if="custom" />
      <template v-else>
        <p v-if="!status" class="hint">Checking…</p>
        <template v-else>
          <ol class="steps" aria-label="Setup progress">
            <li v-for="(label, n) in STEPS" :key="label" :class="{ done: step > n + 1, now: step === n + 1 }">
              <b>{{ step > n + 1 ? '✓' : n + 1 }}</b>{{ label }}
            </li>
          </ol>
          <p v-if="!status.hasToken" class="callout"><slot name="no-token" /></p>
          <div v-if="!status.hasToken || !status.paired" class="row">
            <input v-model="token" type="password" :placeholder="tokenPlaceholder" :disabled="busy" @keydown.enter="saveToken" />
            <button class="primary" :disabled="busy || !token.trim()" @click="saveToken">Save</button>
          </div>
          <div v-if="status.hasToken && status.pending" class="callout ask">
            <span><strong>{{ status.pending.name }}</strong> just messaged this bot.</span>
            <button class="primary" :disabled="busy" @click="emit('approve')">Approve this chat</button>
          </div>
          <template v-else-if="status.hasToken && status.paired">
            <p class="hint">{{ status.connected ? 'Paired and connected - text away.' : (status.problem ?? 'Paired, but not connected yet.') }}</p>
            <div class="actions">
              <button :disabled="busy" @click="emit('refresh')">Check again</button>
              <button class="quiet" :disabled="busy" @click="emit('forget')">Forget this chat</button>
            </div>
          </template>
          <template v-else-if="status.hasToken">
            <p class="hint">{{ status.connected ? 'Connected. Send your bot a message once to pair it.' : (status.problem ?? 'Not connected yet.') }}</p>
            <div class="actions"><button :disabled="busy" @click="emit('refresh')">Check again</button></div>
          </template>
        </template>
      </template>
    </div>

    <button class="guide" @click="emit('guide')">
      <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="9" /><path d="M9.5 9.5a2.5 2.5 0 1 1 3.5 2.3c-.7.4-1 .9-1 1.7M12 17h.01" /></svg>
      How to set this up
    </button>
  </section>
</template>

<style scoped>
.platform {
  --tone: var(--text-dim);
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.platform.tone-ok {
  --tone: var(--ok, #4ade80);
}
.platform.tone-warn {
  --tone: var(--warn, #f5b25a);
}
.platform.tone-idle {
  --tone: var(--accent);
}

.top {
  display: flex;
  align-items: center;
  gap: 14px;
}
.badge {
  display: grid;
  place-items: center;
  flex: none;
  width: 46px;
  height: 46px;
  border-radius: 14px;
  color: var(--accent);
  background: color-mix(in srgb, var(--accent) 14%, transparent);
  border: 1px solid color-mix(in srgb, var(--accent) 35%, transparent);
}
.badge svg {
  width: 24px;
  height: 24px;
}
.name {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 5px;
}
h4 {
  margin: 0;
  font-size: 17px;
  font-weight: 600;
  letter-spacing: 0.04em;
}
.pill {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 2px 10px 2px 8px;
  border-radius: 999px;
  font-size: 11px;
  letter-spacing: 0.06em;
  color: var(--tone);
  background: color-mix(in srgb, var(--tone) 13%, transparent);
}
.pill i {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--tone);
}
.pill.off {
  --tone: var(--text-dim);
}
.pill.ok {
  --tone: var(--ok, #4ade80);
}
.pill.ok i {
  box-shadow: 0 0 8px var(--tone);
}
.pill.warn {
  --tone: var(--warn, #f5b25a);
}
.pill.idle {
  --tone: var(--accent);
}

.switch {
  appearance: none;
  flex: none;
  position: relative;
  width: 44px;
  height: 25px;
  margin: 0;
  border-radius: 999px;
  border: 1px solid var(--line);
  background: color-mix(in srgb, var(--line) 60%, transparent);
  cursor: pointer;
  transition: background 0.18s, border-color 0.18s;
}
.switch::after {
  content: '';
  position: absolute;
  top: 3px;
  left: 3px;
  width: 17px;
  height: 17px;
  border-radius: 50%;
  background: var(--text);
  transition: transform 0.18s;
}
.switch:checked {
  background: var(--accent);
  border-color: var(--accent);
}
.switch:checked::after {
  transform: translateX(19px);
}
.switch:focus-visible,
button:focus-visible,
.row input:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
.switch:disabled {
  opacity: 0.55;
  cursor: progress;
}

.blurb {
  margin: 0;
  font-size: 13px;
  line-height: 1.5;
  color: var(--text-dim);
}
.body {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding-top: 14px;
  border-top: 1px solid var(--line);
}
.hint {
  margin: 0;
  font-size: 12px;
  line-height: 1.5;
  color: var(--text-dim);
}

.steps {
  display: flex;
  gap: 6px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.steps li {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--text-dim);
  white-space: nowrap;
}
.steps b {
  display: grid;
  place-items: center;
  flex: none;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  font-size: 10px;
  font-weight: 600;
  border: 1px solid var(--line);
}
.steps li.now {
  color: var(--text);
}
.steps li.now b {
  color: var(--accent);
  border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 16%, transparent);
}
.steps li.done b {
  color: #0b1020;
  border-color: var(--ok, #4ade80);
  background: var(--ok, #4ade80);
}

.callout {
  margin: 0;
  padding: 10px 12px;
  border-radius: 10px;
  font-size: 12px;
  line-height: 1.5;
  color: var(--text-dim);
  background: color-mix(in srgb, var(--accent) 9%, transparent);
  border: 1px solid color-mix(in srgb, var(--accent) 22%, transparent);
}
.callout.ask {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  color: var(--text);
  background: color-mix(in srgb, var(--warn, #f5b25a) 12%, transparent);
  border-color: color-mix(in srgb, var(--warn, #f5b25a) 40%, transparent);
}
.row {
  display: flex;
  gap: 8px;
}
.row input {
  flex: 1;
  min-width: 0;
}
.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

button:not(.guide):not(.switch) {
  padding: 7px 14px;
  border-radius: 10px;
  border: 1px solid var(--line);
  background: color-mix(in srgb, var(--accent) 8%, transparent);
  color: var(--text);
  font-size: 12px;
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s;
}
button:not(.guide):not(.switch):hover:not(:disabled) {
  border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 18%, transparent);
}
button.primary {
  border-color: var(--accent);
  background: var(--accent);
  color: #0b1020;
  font-weight: 600;
}
button.primary:hover:not(:disabled) {
  background: color-mix(in srgb, var(--accent) 85%, white);
}
button.quiet {
  background: transparent;
  color: var(--text-dim);
}
button:disabled {
  opacity: 0.5;
  cursor: default;
}

/* Slotted content (WhatsApp's body) belongs to the parent's scope, so reach it explicitly. */
.body :deep(.hint) {
  margin: 0;
  font-size: 12px;
  line-height: 1.5;
  color: var(--text-dim);
}
.body :deep(.callout) {
  margin: 0;
  padding: 10px 12px;
  border-radius: 10px;
  font-size: 12px;
  line-height: 1.5;
  color: var(--text-dim);
  background: color-mix(in srgb, var(--accent) 9%, transparent);
  border: 1px solid color-mix(in srgb, var(--accent) 22%, transparent);
}
.body :deep(.callout.warn),
.body :deep(.callout.ask) {
  background: color-mix(in srgb, var(--warn, #f5b25a) 12%, transparent);
  border-color: color-mix(in srgb, var(--warn, #f5b25a) 40%, transparent);
}
.body :deep(.callout.ask) {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  color: var(--text);
}
.body :deep(.actions) {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.body :deep(button) {
  padding: 7px 14px;
  border-radius: 10px;
  border: 1px solid var(--line);
  background: color-mix(in srgb, var(--accent) 8%, transparent);
  color: var(--text);
  font-size: 12px;
  cursor: pointer;
}
.body :deep(button:hover:not(:disabled)) {
  border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 18%, transparent);
}
.body :deep(button.primary) {
  border-color: var(--accent);
  background: var(--accent);
  color: #0b1020;
  font-weight: 600;
}
.body :deep(button.quiet) {
  background: transparent;
  color: var(--text-dim);
}
.body :deep(button:disabled) {
  opacity: 0.5;
  cursor: default;
}
.body :deep(.qr) {
  align-self: center;
  width: 170px;
  height: 170px;
  padding: 8px;
  border-radius: 12px;
  background: #fff;
}

.guide {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  align-self: flex-start;
  margin-top: auto;
  padding: 0;
  border: 0;
  background: none;
  color: var(--accent);
  font-size: 12px;
  letter-spacing: 0.03em;
  cursor: pointer;
}
.guide:hover {
  text-decoration: underline;
  text-underline-offset: 3px;
}
</style>
