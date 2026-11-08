import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// No strictPort: the dev UI moves to the next free port when its first choice is busy.
// scripts/dev.mjs tells Tauri which port it landed on.
export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: { host: 'localhost' },
})
