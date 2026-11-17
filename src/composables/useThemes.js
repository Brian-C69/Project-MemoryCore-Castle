// The themes the member dropped into Castle's settings folder, read once at start-up (App.vue needs
// them to choose the theme anyway) and kept for the whole run, so THEME opens at once instead of
// reading and encoding every wallpaper again. THEME reads them afresh only when the member asks
// (Reload themes) or changes one.
import { ref } from 'vue'
import { userThemes } from '../api/castle.js'

// null until read; then the core's list (each with its wallpaper, or the reason it could not be read)
export const userThemeFolders = ref(null)

export async function loadUserThemes() {
  userThemeFolders.value = await userThemes()
  return userThemeFolders.value
}
