import { ref } from 'vue'
import { devGitDirty } from '../../api/castle.js'

// Close session that saves first, as in the Alice Keep's DEV tab: the project's git tree is read;
// uncommitted files mean `save project` is typed into the companion's session, and the tab closes
// once the tree is clean and she is quiet on two looks in a row (ten minutes at most). A clean tree
// closes at once. A member whose MemoryCore has no projects has no `save project` to call, so the
// tab asks instead (Close anyway / Keep it open).
//
// slot(): the tab's slot; say(text): types into her session; busy / running: her state;
// canSave(): the member's MemoryCore keeps projects; close(): actually close.
export function useSaveClose({ slot, say, busy, running, canSave, close }) {
  // null, or { note, stuck, ask } while saving before the close (ask: the question sheet instead)
  const saving = ref(null)
  let timer = null
  let quiet = 0
  let until = 0

  /** True with uncommitted files, false when clean, null when git can't say. */
  async function dirty() {
    try {
      return await devGitDirty(slot())
    } catch {
      return null
    }
  }

  function stop() {
    clearInterval(timer)
    timer = null
  }

  async function check() {
    if (Date.now() > until) {
      stop()
      saving.value = { note: 'Still saving - the session stays open.', stuck: true, ask: false }
      return
    }
    const now = await dirty()
    if (!timer) return
    quiet = now === false && !busy.value ? quiet + 1 : 0
    if (quiet >= 2) {
      stop()
      saving.value = null
      close()
    }
  }

  async function requestClose() {
    if (timer) return
    saving.value = null
    const changed = await dirty()
    // clean, unknown, or asleep (nothing to save from): close as before
    if (!changed || !running.value) return close()
    if (!canSave()) {
      saving.value = { note: 'This project has changes that are not committed yet.', stuck: false, ask: true }
      return
    }
    say('save project')
    saving.value = { note: 'Saving the project before closing...', stuck: false, ask: false }
    quiet = 0
    until = Date.now() + 10 * 60 * 1000
    timer = setInterval(check, 3000)
  }

  function closeAnyway() {
    stop()
    saving.value = null
    close()
  }

  function dismiss() {
    stop()
    saving.value = null
  }

  // her session ended on its own: nothing to wait for
  function sessionEnded() {
    stop()
    saving.value = null
  }

  return { saving, requestClose, closeAnyway, dismiss, sessionEnded }
}
