<script setup>
import { ref } from 'vue'

// The rooms across the top. Later phases fill each one and hide the rooms whose
// MemoryCore feature is not installed.
const tabs = [
  { id: 'home', label: 'HOME' },
  { id: 'apps', label: 'APPS' },
  { id: 'diary', label: 'DIARY' },
  { id: 'projects', label: 'PROJECTS' },
  { id: 'library', label: 'LIBRARY' },
  { id: 'studio', label: 'STUDIO' },
  { id: 'story', label: 'STORY' },
  { id: 'world', label: 'WORLD' },
  { id: 'chat', label: 'CHAT' },
]
const active = ref('home')
</script>

<template>
  <div class="castle">
    <header class="topbar">
      <div class="brand"><span class="gem">◆</span> MEMORYCORE</div>
      <nav class="tabs">
        <button
          v-for="tab in tabs"
          :key="tab.id"
          :class="{ active: active === tab.id }"
          @click="active = tab.id"
        >{{ tab.label }}</button>
      </nav>
      <button class="gear" title="Settings">⚙</button>
    </header>
    <main class="room">
      <p class="placeholder">{{ tabs.find((t) => t.id === active).label }}</p>
    </main>
  </div>
</template>

<style>
:root {
  --bg: #050b1f;
  --ink: #e6eeff;
  --ink-soft: #9fb2d9;
  --line: rgba(150, 190, 255, 0.25);
  --accent: #8fb8ff;
  font-family: 'Segoe UI', system-ui, sans-serif;
  color: var(--ink);
  background: var(--bg);
}
html, body, #app { margin: 0; height: 100%; overflow: hidden; }
.castle { display: flex; flex-direction: column; height: 100%;
  background: radial-gradient(ellipse at 50% 120%, #123068 0%, #08122e 45%, #040817 100%); }
.topbar { display: flex; align-items: center; gap: 32px; height: 56px; padding: 0 24px;
  border-bottom: 1px solid var(--line); }
.brand { letter-spacing: 0.28em; font-size: 15px; white-space: nowrap; }
.gem { color: var(--accent); margin-right: 8px; }
.tabs { display: flex; gap: 6px; flex: 1; }
.tabs button, .gear { background: none; border: none; color: var(--ink-soft); font: inherit;
  font-size: 13px; letter-spacing: 0.12em; padding: 18px 14px; cursor: pointer;
  border-bottom: 2px solid transparent; }
.tabs button.active { color: var(--ink); border-bottom-color: var(--accent); }
.gear { font-size: 18px; }
.room { flex: 1; display: grid; place-items: center; }
.placeholder { color: var(--ink-soft); letter-spacing: 0.3em; }
</style>
