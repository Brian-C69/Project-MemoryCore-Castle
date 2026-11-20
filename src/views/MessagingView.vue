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
5. Come back to this card - it will show who just messaged it. Press **Approve this chat**.

From now on, texting that number reaches your companion directly, and the answer comes back on WhatsApp.

## Good to know

- Once paired, you won't be asked to scan the QR again unless you get logged out on the phone.
- Only the one chat you approve gets answers.
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
async function approveWhatsapp() {
  await attempt('Approving…', whatsappBridgeApprove)
  await loadWhatsapp()
}
async function forgetWhatsapp() {
  await attempt('Forgetting…', whatsappBridgeForget)
  await loadWhatsapp()
}
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
            <p v-else class="hint">Waiting for a QR code…</p>
          </template>
          <div v-else-if="whatsapp.pending" class="callout ask">
            <span><strong>{{ whatsapp.pending.name }}</strong> just messaged this number.</span>
            <button class="primary" :disabled="!!busy" @click="approveWhatsapp">Approve this chat</button>
          </div>
          <template v-else>
            <p class="hint">{{ whatsapp.connected ? 'Paired and connected - text away.' : (whatsapp.problem ?? 'Paired, but not connected yet.') }}</p>
            <div class="actions">
              <button :disabled="!!busy" @click="loadWhatsapp">Check again</button>
              <button class="quiet" :disabled="!!busy" @click="forgetWhatsapp">Forget this chat</button>
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
</style>
