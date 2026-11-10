// Switching providers hands the new one the saved memory and the recent dialogue, nothing else.
// So when a session is running, Castle asks first: save what you want kept, or switch without.

// The save buttons offered before a switch, from the features the memory folder holds - the same
// commands as the quick-action buttons, so each does exactly what clicking it would.
export function saveChoices(features = {}) {
  return [
    { label: 'Save diary', command: 'save diary' },
    ...(features.projects ? [{ label: 'Save project', command: 'save project' }] : []),
    ...(features.saveMemory ? [{ label: 'Save memory', command: 'save memory' }] : []),
  ]
}

// Whether choosing `target` needs the question: only when a session is running and it is a real
// change of provider. With no session there is nothing to lose.
export function needsSavePrompt({ running, target, current }) {
  return Boolean(running && target && target !== current)
}

// Whether a queued "save, then switch" may actually proceed. The dialog only opens while a session
// is running, but the member can take a moment to answer it; `write()` is a silent no-op once
// nothing is running (Chat relies on that elsewhere), so without this check the switch would go
// ahead as if the save had happened, when nothing was ever typed.
export function canSaveNow({ running }) {
  return Boolean(running)
}
