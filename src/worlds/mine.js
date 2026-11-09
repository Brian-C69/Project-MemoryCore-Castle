// The member's own worlds, read from their memory folder and made into registry entries.
import { convertFileSrc } from '@tauri-apps/api/core'
import { worldsMine } from '../api/castle.js'
import { fromMine } from './registry.js'

export async function loadMine() {
  try {
    return (await worldsMine()).map((item) => fromMine(item, convertFileSrc))
  } catch {
    return [] // outside Castle, or no memory folder yet
  }
}
