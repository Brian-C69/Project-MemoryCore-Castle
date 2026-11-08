<script setup>
// One card component for HOME's memory widgets, so every card shares one look: TODAY,
// PROJECTS, DIARY, GROWTH and BACKUP. Each shows a friendly empty line when the member's
// memory has nothing there yet (a fresh MemoryCore), never an error.
const props = defineProps({
  kind: { type: String, required: true }, // today | projects | diary | growth | backup
  data: { type: Object, default: null }, // HomeData from memory_home
  backup: { type: Object, default: null }, // BackupStatus
})
defineEmits(['open', 'backup'])

const TITLES = { today: 'TODAY', projects: 'PROJECTS', diary: 'DIARY', growth: 'GROWTH', backup: 'BACKUP' }
const ICONS = { today: '▦', projects: '▤', diary: '❒', growth: '↗', backup: '⇪' }

function ago(iso) {
  if (!iso) return 'never'
  const ms = Date.now() - new Date(iso).getTime()
  if (Number.isNaN(ms)) return iso
  const minutes = Math.round(ms / 60000)
  if (minutes < 60) return `${Math.max(1, minutes)} min ago`
  const hours = Math.round(minutes / 60)
  if (hours < 48) return `${hours} h ago`
  return `${Math.round(hours / 24)} days ago`
}

function due(date) {
  if (!date) return ''
  const when = new Date(`${date}T00:00:00`)
  return Number.isNaN(when.getTime()) ? date : when.toLocaleDateString('en-US', { month: 'short', day: 'numeric' })
}
</script>

<template>
  <section class="widget card" :class="kind">
    <h3><span class="icon">{{ ICONS[kind] }}</span>{{ TITLES[kind] }}</h3>

    <template v-if="kind === 'today'">
      <ul v-if="data?.reminders?.length">
        <li v-for="r in data.reminders.slice(0, 4)" :key="r.title" :title="r.detail">
          <span class="dot">○</span><span class="text">{{ r.title }}</span><span class="side">{{ due(r.due) }}</span>
        </li>
      </ul>
      <p v-else class="empty">{{ data?.features?.reminders ? 'Nothing open - a calm day.' : 'Ask your companion to remind you of something.' }}</p>
    </template>

    <template v-else-if="kind === 'projects'">
      <ul v-if="data?.projects?.length">
        <li v-for="(p, i) in data.projects.slice(0, 4)" :key="p.name" class="link" @click="$emit('open', p.file)">
          <span class="num">{{ i + 1 }}</span><span class="text">{{ p.name }}</span><span class="side">›</span>
        </li>
      </ul>
      <p v-else class="empty">No projects yet.</p>
    </template>

    <template v-else-if="kind === 'diary'">
      <p v-if="data?.diary" class="link diary-line" @click="$emit('open', data.diary.file)">
        <span class="text">{{ data.growth?.days ? `Day ${data.growth.days} - ` : '' }}{{ data.diary.title }}</span><span class="side">›</span>
      </p>
      <p v-else class="empty">Press Save diary after a talk to write the first page.</p>
    </template>

    <template v-else-if="kind === 'growth'">
      <p class="growth-line">
        <span>{{ data?.growth?.memories ?? 0 }} memories</span>
        <span class="sep">·</span>
        <span>{{ data?.growth?.days ?? 0 }} days</span>
        <span class="sep">·</span>
        <span>{{ data?.growth?.skills ?? 0 }} skills</span>
      </p>
    </template>

    <template v-else-if="kind === 'backup'">
      <p v-if="!backup" class="empty">Checking…</p>
      <template v-else-if="backup.ready">
        <p class="facts">{{ ago(backup.lastBackup) }} · {{ backup.changed }} changed</p>
        <button @click="$emit('backup')">Back up now</button>
      </template>
      <template v-else>
        <p class="empty">Not set up yet.</p>
        <button @click="$emit('backup', 'setup')">Set up backup</button>
      </template>
    </template>
  </section>
</template>

<style scoped>
.card {
  padding: 14px 18px;
  box-sizing: border-box;
}
h3 {
  margin: 0 0 10px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--line);
  font-size: 13px;
  font-weight: 500;
  letter-spacing: 0.14em;
}
.icon {
  margin-right: 10px;
  color: var(--accent);
}
ul {
  display: grid;
  gap: 8px;
  margin: 0;
  padding: 0;
  list-style: none;
}
li,
.diary-line {
  display: flex;
  gap: 10px;
  align-items: center;
  margin: 0;
  font-size: 14px;
}
.text {
  flex: 1;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.side {
  font-size: 12px;
  color: var(--text-dim);
}
.dot {
  color: var(--text-dim);
}
.num {
  display: grid;
  place-items: center;
  width: 22px;
  height: 22px;
  border: 1px solid var(--line);
  border-radius: 50%;
  font-size: 12px;
}
.link {
  cursor: pointer;
}
.link:hover .text {
  color: var(--accent);
}
.empty,
.facts {
  margin: 0 0 6px;
  font-size: 13px;
  color: var(--text-dim);
}
.growth-line {
  display: flex;
  gap: 10px;
  margin: 0;
  font-size: 15px;
}
.sep {
  color: var(--text-dim);
}
button {
  padding: 5px 12px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--hover);
  color: var(--text);
  font: inherit;
  font-size: 13px;
  cursor: pointer;
}
</style>
