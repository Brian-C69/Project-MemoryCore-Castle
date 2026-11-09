import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// No strictPort: the dev UI moves to the next free port when its first choice is busy.
// scripts/dev.mjs tells Tauri which port it landed on.
export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: { host: 'localhost' },
  // the 3D worlds' model loader is found only when a world window opens; bundling it up front
  // keeps the dev server from reloading every window at that moment
  optimizeDeps: { include: ['three/examples/jsm/loaders/GLTFLoader.js', 'three/examples/jsm/libs/meshopt_decoder.module.js'] },
})
