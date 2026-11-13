// The DEV tab's FILES, talking to Castle's Rust core. Every path is relative to the tab's project
// ("" = the top); the core finds the project from the slot and keeps every path inside it.
import { invoke } from '@tauri-apps/api/core'

// -> { entries: [{ name, rel, dir, size, ignored, private }], more }
export const devTree = (slot, rel = '') => invoke('dev_tree', { slot, rel })
// -> { kind: 'text', text, size } | { kind: 'picture', url, size } | { kind: 'note', note, size }
export const devFile = (slot, rel) => invoke('dev_file', { slot, rel })
// Windows Explorer opened with the file selected.
export const devReveal = (slot, rel) => invoke('dev_reveal', { slot, rel })
