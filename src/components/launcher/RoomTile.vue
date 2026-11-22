<script setup>
// One room in the launcher: a large rounded square (the room's initial until its icon exists), the name
// under it, its hint in the hover tooltip (nothing slides open under the name), a pin that puts the room in the top bar, and a badge when
// something is waiting there.
import { computed } from 'vue'
import { initialOf } from '../../launcher.js'
import { roomIcon } from '../../roomIcons.js'

const props = defineProps({
  tab: { type: Object, required: true },
  hint: { type: String, default: '' },
  pinned: { type: Boolean, default: false },
  // HOME has no pin: it is always in the top bar
  pinnable: { type: Boolean, default: true },
  // what is waiting there: { count, tone: 'bad' | 'warn' | 'info', text } (a count of 0 is a plain dot)
  badge: { type: Object, default: null },
  // the room that is open now
  current: { type: Boolean, default: false },
  // where it sits, for the arrow keys
  group: { type: Number, default: 0 },
  index: { type: Number, default: 0 },
})
const emit = defineEmits(['open', 'pin'])
// Odoo-style names: "My Profile", not "MY PROFILE"
const pretty = computed(() => props.tab.label.toLowerCase().replace(/(^|\s)\S/g, (c) => c.toUpperCase()))
const letter = computed(() => initialOf(props.tab.label))
const art = computed(() => roomIcon(props.tab.id))
const label = computed(() => (props.badge?.text ? `Open ${props.tab.label}. ${props.badge.text}` : `Open ${props.tab.label}`))
const shown = computed(() => (props.badge?.count > 99 ? '99+' : props.badge?.count || ''))
</script>

<template>
  <div class="tile" :class="{ current }">
    <button type="button" class="open" :data-g="group" :data-i="index" :aria-label="label" :aria-current="current ? 'page' : undefined" @click="emit('open', tab.id)">
      <span class="icon" aria-hidden="true"><span v-if="art" class="room-art" v-html="art"></span><span v-else>{{ letter }}</span><span v-if="badge" class="badge" :class="[badge.tone, { plain: !badge.count }]">{{ shown }}</span></span>
      <span class="name">{{ pretty }}</span>
    </button>
    <button v-if="pinnable" type="button" class="pin" :class="{ on: pinned }" :aria-pressed="pinned" :aria-label="pinned ? `Take ${tab.label} out of the top bar` : `Pin ${tab.label} to the top bar`" :title="pinned ? 'In the top bar. Click to take it out.' : 'Pin to the top bar'" tabindex="-1" @click="emit('pin', { id: tab.id, on: !pinned })">
      <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round" aria-hidden="true"><path d="M10.5 1.8l3.7 3.7-2.1.6-2.6 2.6.2 3.1-1 1-2.8-2.8-4 4M6.6 4.4l-.3-.2 1-1 3.1.2" /></svg>
    </button>
  </div>
</template>

<style scoped>
.tile { --tile: 72px; position: relative; min-width: 0; }
.open {
  display: flex; flex-direction: column; align-items: center; gap: 8px; width: 100%; padding: 4px 2px 6px;
  border: 0; border-radius: 14px; background: none; color: var(--text); font: inherit; cursor: pointer;
  transition: transform 0.15s;
}
.open:hover { transform: translateY(-2px); }
.open:focus-visible { outline: 2px solid var(--accent); outline-offset: 3px; }
/* Both surface and monochrome artwork follow the active Castle theme. */
.icon {
  position: relative; display: grid; place-items: center; width: var(--tile); height: var(--tile); border-radius: calc(var(--tile) * 0.2);
  /* A nearly solid theme base keeps bright wallpaper behind the accent artwork. */
  background: var(--background);
  background: color-mix(in srgb, var(--background) 92%, transparent);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.18), 0 0 0 1px var(--line);
  color: var(--accent); font-size: calc(var(--tile) * 0.42); font-weight: 700;
  transition: box-shadow 0.15s;
}
.room-art { display: grid; place-items: center; width: 84%; height: 84%; }
.room-art :deep(svg) { display: block; width: 100%; height: 100%; }
.open:hover .icon { box-shadow: 0 6px 14px rgba(0, 0, 0, 0.25), 0 0 0 1px var(--line); }
.tile.current .icon { box-shadow: 0 0 0 2px var(--accent), 0 1px 3px rgba(0, 0, 0, 0.18); }
.badge {
  position: absolute; top: -6px; right: -6px; display: grid; place-items: center; min-width: 20px; height: 20px; padding: 0 5px; box-sizing: border-box;
  border-radius: 10px; font-size: 11px; font-weight: 700; line-height: 1; color: #1a1a1a; background: var(--accent);
  box-shadow: 0 0 0 2px var(--panel-background);
}
.badge.plain { min-width: 10px; width: 10px; height: 10px; padding: 0; top: -3px; right: -3px; border-radius: 50%; }
.badge.bad { background: #ef9393; }
.badge.warn { background: #e0b25c; }
.name { font-size: 13px; font-weight: 500; text-align: center; line-height: 1.25; overflow-wrap: anywhere; text-shadow: 0 1px 4px rgba(0, 0, 0, 0.85), 0 0 2px rgba(0, 0, 0, 0.6); }
.pin {
  position: absolute; top: -6px; right: calc(50% - var(--tile) / 2 - 8px); display: grid; place-items: center; width: 22px; height: 22px; padding: 0;
  border: 0; border-radius: 50%; background: var(--panel-background); color: var(--text-dim); cursor: pointer; opacity: 0;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3); transition: opacity 0.15s, color 0.15s;
}
.tile:hover .pin, .tile:focus-within .pin, .pin.on { opacity: 1; }
.pin.on { color: var(--accent); }
.pin:hover { color: var(--text); }
.pin:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; opacity: 1; }
</style>
