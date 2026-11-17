// WORLD's facts, kept for the whole run: the settings, the screens plugged in, the member's own worlds,
// whether each world's pieces are all there, and the crystal's limit line. Castle reads them at
// start-up behind the start-up screen, so WORLD opens with its cards and screens already there; the
// room then reads them again quietly (and every few seconds while it is shown) and swaps in anything
// new. No world is drawn here: this is only the facts.
import { ref } from 'vue'
import { cliQuota, getSettings, worldScreens } from '../api/castle.js'
import { loadMine } from '../worlds/mine.js'
import { allWorlds } from '../worlds/registry.js'
import { readinessOf } from '../worlds/readiness.js'

export const worldSettings = ref(null)
export const worldScreensNow = ref([])
export const worldMine = ref([])
export const worldStates = ref({}) // world id -> { have, total, ready }
export const worldQuota = ref(undefined) // undefined = not asked yet, null = unknown

// Settings, screens and the member's own worlds (throws outside Castle's window).
export async function readWorldBasics() {
  worldSettings.value = (await getSettings()).settings
  worldScreensNow.value = await worldScreens()
  worldMine.value = await loadMine()
}

export async function checkWorldStates(fresh = false) {
  const next = { ...worldStates.value }
  await Promise.all(allWorlds(worldMine.value).filter((w) => w.engine === 'glb').map(async (w) => (next[w.id] = await readinessOf(w, { fresh }))))
  worldStates.value = next
}

export async function readWorldQuota() {
  worldQuota.value = await cliQuota().catch(() => null)
}

export async function prepareWorld() {
  try {
    await readWorldBasics()
    await checkWorldStates()
    await readWorldQuota()
  } catch {
    // outside Castle's window: the room reads them when it opens
  }
}
