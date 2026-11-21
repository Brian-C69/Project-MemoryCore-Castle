<script setup>
// MESSAGING: MemoryCore in your pocket. One card per platform; each card is its own setup -
// token, pairing, status - so a new platform just adds another card, never a reshuffle.
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import QRCode from 'qrcode/lib/browser.js'
import {
  getSettings,
  saveSettings,
  keySave,
  telegramBridgeStatus,
  telegramBridgeApprove,
  telegramBridgeForget,
  discordBridgeStatus,
  discordBridgeApprove,
  discordBridgeForget,
  whatsappBridgeStatus,
  whatsappBridgeApprove,
  whatsappBridgeForget,
  whatsappBridgeMakeMe,
  whatsappBridgeRemoveGuest,
  whatsappBridgeUnpair,
} from '../api/castle.js'
import PlatformBridgeCard from '../components/PlatformBridgeCard.vue'
import ModalSheet from '../components/ModalSheet.vue'
import MarkdownText from '../components/MarkdownText.vue'

defineProps({ companion: { type: String, default: '' } })

const GUIDES = {
  telegram: {
    title: 'Setting up Telegram',
    body: `## What you'll need

Just the Telegram app on your phone or computer - nothing to install, nothing to sign up for.

## Steps

1. Open Telegram and search for **@BotFather**. This is Telegram's own official helper for making bots - it's a real, verified account, not a stranger.
2. Send it the message **/newbot**.
3. It asks for a name (anything you like) and then a username, which must end in "bot" - for example \`mira_pocket_bot\`.
4. BotFather replies with a long string of letters and numbers. This is your bot's **token** - think of it as its password. Copy the whole thing.
5. Come back to this card, paste that token into the box, and press **Save**.
6. Now find your new bot in Telegram (search for the username you picked) and send it any message, like "hi".
7. Come back to this card again - it will show that someone just messaged it. Press **Approve this chat**.

That's it. From now on, anything you text that bot reaches your companion directly, and the reply comes back on Telegram.

## Good to know

- Only the one chat you approve can talk to your companion this way. Anyone else who finds your bot just gets a polite "not approved yet" message.
- Nothing you text here shows up in Castle's shared chat room.`,
  },
  discord: {
    title: 'Setting up Discord',
    body: `## What you'll need

A free Discord account, and about five minutes on Discord's own website.

## Steps

1. Go to **discord.com/developers/applications** in a web browser and sign in with your Discord account.
2. Press **New Application**, give it any name, and create it.
3. On the left-hand menu, click **Bot**. Press **Reset Token** (or **Copy** if one is already showing) to get the bot's **token** - its password, so keep it private.
4. Still on the Bot page, find **Message Content Intent** and switch it **on**. This step matters: without it, Discord hides the actual words of every message before your bot can see them, and nothing will work.
5. Come back to this card, paste that token into the box, and press **Save**.
6. To reach the bot, it needs to be somewhere you can message it: under "Installation", generate an invite link (OAuth2 > URL Generator, tick "bot"), open that link, and add it to a server you're in - your own private server is fine if you don't have one already.
7. Click the bot's name and send it a direct message.
8. Come back to this card - it will show who just messaged it. Press **Approve this chat**.

From now on, DMing that bot reaches your companion directly, and the answer comes back in Discord.

## Good to know

- Only the one chat you approve gets answers.
- Nothing you text here shows up in Castle's shared chat room.`,
  },
  whatsapp: {
    title: 'Setting up WhatsApp',
    body: `## Before you start

WhatsApp doesn't officially support this, so it works a little differently from the other two - and there's a real chance the number used here gets limited or banned by WhatsApp for it. **Use a second number you don't mind losing, never your main phone number.** A cheap spare SIM, or a second WhatsApp-capable device, is the safe way to do this.

## Steps

1. Switch this card **On**.
2. After a moment, a QR code appears on the card - the same idea as scanning in to WhatsApp Web.
3. On the **secondary** phone, open WhatsApp, go to **Settings > Linked Devices > Link a Device**, and scan that QR code.
4. Once it's linked, send a message to that WhatsApp number from whichever phone you actually want to text your companion from.
5. Come back to this card - it will show who just messaged it. Press **This is me**.

From now on, texting that number reaches your companion directly, and the answer comes back on WhatsApp.

## Friends and family (guests)

Someone else can message that number too, once you approve them. When they message it, the card shows who it is: press **Approve as a guest**. A guest is answered at once, but **not** by your companion's real session. A separate guest-only run answers instead. It knows your companion's public card and nothing private about you, and it has no tools. After each answer you get a WhatsApp message on your own number saying who wrote, what they said and what was answered.

- **Approved numbers** lists you (with a **Me** chip) and your guests. **Remove** lets a guest go; **Make me** makes that number yours, for the day your own number changes.
- **Not this one** leaves a chat unanswered.

## Good to know

- Once paired, you won't be asked to scan the QR again unless you get logged out on the phone.
- If that number is banned or logged out, press **Pair a new number** and scan again with another one. You and every guest stay approved.
- Your reactions on your companion's replies (a heart, a laugh) show on its bubble in Chat.
- Nothing you text here shows up in Castle's shared chat room.`,
  },
}
const guideOpen = ref('')

const settings = ref(null)
const telegram = ref(null)
const discord = ref(null)
const whatsapp = ref(null)
const whatsappQrImage = ref('')
const busy = ref('')
const problem = ref('')
const done = ref('')
let poll = null

async function attempt(label, work) {
  problem.value = ''
  done.value = ''
  busy.value = label
  try {
    return await work()
  } catch (reason) {
    problem.value = String(reason)
    return null
  } finally {
    busy.value = ''
  }
}

async function loadTelegram() {
  telegram.value = settings.value?.telegramBridgeOn ? await telegramBridgeStatus().catch(() => null) : null
}
async function toggleTelegram(on) {
  const saved = await attempt('Saving…', () => saveSettings({ ...settings.value, telegramBridgeOn: on }))
  if (saved) {
    settings.value = saved.settings
    setTimeout(loadTelegram, 2000)
  }
}
async function saveTelegramToken(token) {
  const saved = await attempt('Saving…', () => keySave('telegram-bridge', token))
  if (saved) {
    done.value = 'Token saved.'
    await loadTelegram()
  }
}
async function approveTelegram() {
  await attempt('Approving…', telegramBridgeApprove)
  await loadTelegram()
}
async function forgetTelegram() {
  await attempt('Forgetting…', telegramBridgeForget)
  await loadTelegram()
}

async function loadDiscord() {
  discord.value = settings.value?.discordBridgeOn ? await discordBridgeStatus().catch(() => null) : null
}
async function toggleDiscord(on) {
  const saved = await attempt('Saving…', () => saveSettings({ ...settings.value, discordBridgeOn: on }))
  if (saved) {
    settings.value = saved.settings
    setTimeout(loadDiscord, 2000)
  }
}
async function saveDiscordToken(token) {
  const saved = await attempt('Saving…', () => keySave('discord-bridge', token))
  if (saved) {
    done.value = 'Token saved.'
    await loadDiscord()
  }
}
async function approveDiscord() {
  await attempt('Approving…', discordBridgeApprove)
  await loadDiscord()
}
async function forgetDiscord() {
  await attempt('Forgetting…', discordBridgeForget)
  await loadDiscord()
}

async function loadWhatsapp() {
  whatsapp.value = settings.value?.whatsappBridgeOn ? await whatsappBridgeStatus().catch(() => null) : null
}
async function toggleWhatsapp(on) {
  const saved = await attempt('Saving…', () => saveSettings({ ...settings.value, whatsappBridgeOn: on }))
  if (saved) {
    settings.value = saved.settings
    setTimeout(loadWhatsapp, 2000)
  }
}
async function approveWhatsapp(asWho) {
  await attempt(asWho === 'no' ? 'Leaving it…' : 'Approving…', () => whatsappBridgeApprove(asWho))
  await loadWhatsapp()
}
async function forgetWhatsapp() {
  await attempt('Forgetting…', whatsappBridgeForget)
  await loadWhatsapp()
}
async function makeMeWhatsapp(chatId) {
  await attempt('Saving…', () => whatsappBridgeMakeMe(chatId))
  await loadWhatsapp()
}
async function removeGuestWhatsapp(chatId) {
  await attempt('Removing…', () => whatsappBridgeRemoveGuest(chatId))
  await loadWhatsapp()
}
async function unpairWhatsapp() {
  const ok = await attempt('Unpairing…', whatsappBridgeUnpair)
  if (ok !== null) done.value = 'The old number is unpaired. Scan the new QR code; everyone you approved stays approved.'
  await loadWhatsapp()
}
const whatsappPeople = computed(() => {
  const w = whatsapp.value
  if (!w) return []
  return [...(w.me ? [{ ...w.me, me: true }] : []), ...(w.guests ?? []).map((g) => ({ ...g, me: false }))]
})
const whatsappPill = computed(() => {
  const w = whatsapp.value
  if (!settings.value?.whatsappBridgeOn) return { text: 'Off', tone: 'off' }
  if (!w) return { text: 'Starting…', tone: 'idle' }
  if (!w.paired) return { text: 'Scan the code', tone: 'warn' }
  if (w.pending) return { text: 'Waiting for you', tone: 'warn' }
  return w.connected ? { text: 'Connected', tone: 'ok' } : { text: 'Not connected', tone: 'warn' }
})
watch(
  () => whatsapp.value?.qr,
  async (qr) => {
    whatsappQrImage.value = qr ? await QRCode.toDataURL(qr, { margin: 1, width: 220 }).catch(() => '') : ''
  },
)

onMounted(async () => {
  settings.value = (await getSettings()).settings
  await Promise.all([loadTelegram(), loadDiscord(), loadWhatsapp()])
  poll = setInterval(() => {
    if (settings.value?.telegramBridgeOn) loadTelegram()
    if (settings.value?.discordBridgeOn) loadDiscord()
    if (settings.value?.whatsappBridgeOn) loadWhatsapp()
  }, 5000)
})
onBeforeUnmount(() => clearInterval(poll))
</script>

<template>
  <div class="messaging">
    <header class="bar">
      <h2>MESSAGING</h2>
      <span class="spacer"></span>
    </header>
    <p class="lede">Text {{ companion || 'your companion' }} - the same session, memory and tools DEV's Chat has - from your phone. Nothing here ever posts into the shared chat room.</p>
    <p v-if="problem" role="alert" class="problem">{{ problem }}</p>
    <p v-else-if="done" class="hint">{{ done }}</p>
    <div class="cards" v-if="settings">
      <PlatformBridgeCard
        title="Telegram"
        icon="telegram"
        blurb="Make a bot with Telegram's own BotFather, then text it from the Telegram app you already use."
        :on="!!settings.telegramBridgeOn"
        :busy="!!busy"
        :status="telegram"
        @toggle="toggleTelegram"
        @save-token="saveTelegramToken"
        @approve="approveTelegram"
        @forget="forgetTelegram"
        @refresh="loadTelegram"
        @guide="guideOpen = 'telegram'"
      >
        <template #no-token>Make a bot with Telegram's <strong>@BotFather</strong>, then paste its token above.</template>
      </PlatformBridgeCard>
      <PlatformBridgeCard
        title="Discord"
        icon="discord"
        blurb="Add your own bot to a private server, then send it a direct message."
        :on="!!settings.discordBridgeOn"
        :busy="!!busy"
        :status="discord"
        @toggle="toggleDiscord"
        @save-token="saveDiscordToken"
        @approve="approveDiscord"
        @forget="forgetDiscord"
        @refresh="loadDiscord"
        @guide="guideOpen = 'discord'"
      >
        <template #no-token>
          Make a bot in Discord's <strong>Developer Portal</strong>, switch on <strong>Message Content Intent</strong> for it, then paste its token above.
        </template>
      </PlatformBridgeCard>
      <PlatformBridgeCard
        title="WhatsApp"
        icon="whatsapp"
        blurb="Link a spare number the way you would WhatsApp Web, then text your companion from your own phone."
        custom
        :pill-text="whatsappPill.text"
        :pill-tone="whatsappPill.tone"
        :on="!!settings.whatsappBridgeOn"
        :busy="!!busy"
        @toggle="toggleWhatsapp"
        @guide="guideOpen = 'whatsapp'"
      >
        <p v-if="!whatsapp" class="hint">Starting…</p>
        <template v-else>
          <p class="callout warn">Unofficial - WhatsApp has no public API for this. Pair with a <strong>secondary number</strong>, never your main one: it can be restricted or banned.</p>
          <template v-if="!whatsapp.paired">
            <p v-if="whatsappQrImage" class="hint">Scan with WhatsApp's <strong>Linked Devices</strong> on the secondary phone.</p>
            <img v-if="whatsappQrImage" :src="whatsappQrImage" alt="WhatsApp pairing QR code" class="qr" />
            <p v-else class="hint">{{ whatsapp.problem ?? 'Waiting for a QR code…' }}</p>
            <div v-if="!whatsappQrImage && whatsapp.problem" class="actions">
              <button class="quiet" :disabled="!!busy" @click="unpairWhatsapp">Pair a new number</button>
            </div>
          </template>
          <template v-else>
            <div v-if="whatsapp.pending" class="callout ask wa-waiting">
              <span>
                <strong>{{ whatsapp.pending.name }}</strong>
                <span v-if="whatsapp.pending.number" class="wa-number">{{ whatsapp.pending.number }}</span>
                just messaged this number.
              </span>
              <div class="actions">
                <button class="primary" :disabled="!!busy" @click="approveWhatsapp('me')">This is me</button>
                <button :disabled="!!busy || whatsapp.pending.chatId === whatsapp.me?.chatId" @click="approveWhatsapp('guest')">Approve as a guest</button>
                <button class="quiet" :disabled="!!busy" @click="approveWhatsapp('no')">Not this one</button>
              </div>
            </div>
            <p class="hint">{{ whatsapp.connected ? (whatsapp.me ? 'Paired and connected - text away.' : 'Paired and connected. Message this number from your own phone, then press This is me.') : (whatsapp.problem ?? 'Paired, but not connected yet.') }}</p>
            <div v-if="whatsappPeople.length" class="wa-people">
              <p class="wa-title">Approved numbers</p>
              <div v-for="person in whatsappPeople" :key="person.chatId" class="wa-person">
                <span class="wa-name">{{ person.name }}</span>
                <span v-if="person.number" class="wa-number">{{ person.number }}</span>
                <span v-if="person.me" class="wa-chip">Me</span>
                <span class="wa-spacer" />
                <template v-if="person.me">
                  <button class="quiet" :disabled="!!busy" @click="forgetWhatsapp">Forget</button>
                </template>
                <template v-else>
                  <button class="quiet" :disabled="!!busy" @click="makeMeWhatsapp(person.chatId)">Make me</button>
                  <button class="quiet" :disabled="!!busy" @click="removeGuestWhatsapp(person.chatId)">Remove</button>
                </template>
              </div>
              <p v-if="whatsapp.guests?.length" class="wa-note">Guests are answered at once by a guest-only {{ companion || 'companion' }} that knows nothing private, and you get a message about each answer.</p>
            </div>
            <div class="actions">
              <button :disabled="!!busy" @click="loadWhatsapp">Check again</button>
              <button class="quiet" :disabled="!!busy" @click="unpairWhatsapp">Pair a new number</button>
            </div>
          </template>
        </template>
      </PlatformBridgeCard>
    </div>
    <ModalSheet v-if="guideOpen" tag="MESSAGING" :title="GUIDES[guideOpen].title" @close="guideOpen = ''">
      <MarkdownText :text="GUIDES[guideOpen].body" size="reader" />
    </ModalSheet>
  </div>
</template>

<style scoped>
.messaging {
  position: absolute;
  inset: 24px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow: auto;
}
.bar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 12px;
}
h2 {
  margin: 0;
  font-size: 13px;
  font-weight: 500;
  letter-spacing: 0.2em;
  color: var(--accent);
}
.spacer {
  flex: 1;
}
.lede {
  margin: 0;
  color: var(--text-dim);
  max-width: 60em;
}
.cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 20px;
  align-items: stretch;
}
.problem {
  margin: 0;
  font-size: 13px;
  color: var(--warn);
}
.wa-waiting {
  flex-direction: column;
  align-items: flex-start;
  gap: 8px;
}
.wa-people {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.wa-title {
  margin: 0;
  font-size: 11px;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: var(--text-dim);
}
.wa-person {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
}
.wa-name {
  font-weight: 500;
}
.wa-number {
  font-size: 12px;
  color: var(--text-dim);
  font-variant-numeric: tabular-nums;
}
.wa-chip {
  padding: 1px 8px;
  border-radius: 999px;
  font-size: 11px;
  background: color-mix(in srgb, var(--accent) 30%, transparent);
  color: var(--text);
}
.wa-spacer {
  flex: 1;
}
.wa-note {
  margin: 2px 0 0;
  font-size: 12px;
  color: var(--text-dim);
}
</style>
