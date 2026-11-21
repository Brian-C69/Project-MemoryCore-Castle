// What each Settings row says about its last save: "Saved ✓" for a moment after it worked, or a red
// line under the row when it failed (the control itself keeps the old value, because the page only
// takes the new settings once Castle has kept them). Also which row a deep link lit up.
// SettingsView makes one and provides it; SettingRow and SettingGroup read it by their id.
import { reactive } from 'vue'

export const ROW_STATUS = Symbol('settings-row-status')
export const SAVED_MS = 1500
export const LIT_MS = 2400

export function createRowStatus({ setTimer = setTimeout, clearTimer = clearTimeout } = {}) {
  const state = reactive({ saved: {}, errors: {}, lit: '' })
  const timers = {}
  let litTimer = null

  function saved(id) {
    if (!id) return
    delete state.errors[id]
    state.saved[id] = true
    clearTimer(timers[id])
    timers[id] = setTimer(() => {
      state.saved[id] = false
    }, SAVED_MS)
  }

  // `lead` names what did not happen ("Not saved", "Not changed", "Not copied")
  function failed(id, reason, lead = 'Not saved') {
    if (!id) return
    clearTimer(timers[id])
    state.saved[id] = false
    const said = String(reason?.message ?? reason ?? '').trim()
    state.errors[id] = said ? `${lead}: ${said}` : `${lead}. Try again.`
  }

  function clear(id) {
    if (id) delete state.errors[id]
  }

  /** Runs one row's save: flashes Saved on success, keeps the reason under the row on failure. */
  async function run(id, work) {
    clear(id)
    try {
      const result = await work()
      saved(id)
      return result
    } catch (reason) {
      failed(id, reason)
      return null
    }
  }

  function light(id) {
    clearTimer(litTimer)
    state.lit = id || ''
    if (id) litTimer = setTimer(() => (state.lit = ''), LIT_MS)
  }

  return { state, saved, failed, clear, run, light }
}
