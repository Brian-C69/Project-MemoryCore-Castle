<script setup>
// One diary entry, dressed in one of three ways (Kiyo's choice, Nov 10 2026):
//   letter   - "a letter under starlight": a narrow serif column on a lit sheet (the Book)
//   journal  - "the constellation journal": a date block, a pull quote, a thread of days (the Calendar)
//   notebook - "the handwritten night journal": ruled lines, a date stamp, margin marks (the List)
// An entry of plain prose is set paragraph by paragraph; one that holds markdown structure is
// handed to the markdown reader inside the same page.
import { computed } from 'vue'
import MarkdownText from './MarkdownText.vue'
import { dateParts, emblemPoints, longDate, neighbours, plainParagraphs, pullQuote, readMinutes, shortDate } from './diaryviews.js'

const props = defineProps({
  design: { type: String, default: 'letter' }, // letter | journal | notebook
  page: { type: Object, default: null }, // { title, file, date }
  text: { type: String, default: '' },
  problem: { type: String, default: '' },
  // every diary page, oldest first (for the journal's thread of days)
  pages: { type: Array, default: () => [] },
  companion: { type: String, default: '' },
  // the Book's pages are narrow: smaller type, no side thread
  compact: { type: Boolean, default: false },
})
const emit = defineEmits(['open'])

const paragraphs = computed(() => plainParagraphs(props.text))
const quote = computed(() => (props.design === 'journal' && paragraphs.value ? pullQuote(paragraphs.value) : null))
// the quote is shown once, just before the paragraph it comes from (which keeps its sentence)
const quoteAt = computed(() => (quote.value ? paragraphs.value.findIndex((p) => p.includes(quote.value)) : -1))
const parts = computed(() => dateParts(props.page?.date))
const around = computed(() => neighbours(props.pages, props.page?.file))
const minutes = computed(() => readMinutes(props.text))
const by = computed(() => props.companion.trim())

// the journal's emblem: five stars joined by a hairline, the same for the same date
const emblem = computed(() => {
  const points = emblemPoints(props.page?.date ?? props.page?.title ?? '')
  const at = (p) => `${(p.x * 44 + 2).toFixed(1)},${(p.y * 30 + 2).toFixed(1)}`
  return { line: points.map(at).join(' '), stars: points.map((p) => ({ cx: (p.x * 44 + 2).toFixed(1), cy: (p.y * 30 + 2).toFixed(1) })) }
})

// the notebook's margin marks, one beside each paragraph in turn
const MARKS = ['sprout', 'star', 'heart']
const markOf = (i) => MARKS[i % MARKS.length]
</script>

<template>
  <article v-if="page" class="diary-page" :class="[design, { compact }]">
    <!-- A: a letter under starlight -->
    <template v-if="design === 'letter'">
      <div class="sheet">
        <p class="when">{{ longDate(page.date) || 'Earlier' }}</p>
        <svg class="rule" viewBox="0 0 120 10" aria-hidden="true">
          <polyline points="6,5 33,5 60,5 87,5 114,5" />
          <circle v-for="x in [6, 33, 60, 87, 114]" :key="x" :cx="x" cy="5" r="1.6" />
        </svg>
        <h1>{{ page.title }}</h1>
        <p v-if="problem" class="problem">{{ problem }}</p>
        <div v-else-if="paragraphs" class="words">
          <p v-for="(p, i) in paragraphs" :key="i" :class="{ first: i === 0 }">{{ p }}</p>
        </div>
        <div v-else class="words md"><MarkdownText :text="text" /></div>
        <p class="ornament" aria-hidden="true">✦</p>
        <p class="foot">{{ by ? `written by ${by}` : 'from the diary' }} · a {{ minutes }} minute read</p>
      </div>
    </template>

    <!-- B: the constellation journal -->
    <template v-else-if="design === 'journal'">
      <div class="main">
        <header>
          <div v-if="parts" class="date-block">
            <span class="day">{{ parts.day }}</span>
            <span class="stack"><b>{{ parts.month }}</b><i>{{ parts.weekday }}</i></span>
          </div>
          <svg class="emblem" viewBox="0 0 48 34" aria-hidden="true">
            <polyline :points="emblem.line" />
            <circle v-for="(s, i) in emblem.stars" :key="i" :cx="s.cx" :cy="s.cy" r="1.4" />
          </svg>
        </header>
        <h1>{{ page.title }}</h1>
        <p v-if="problem" class="problem">{{ problem }}</p>
        <div v-else-if="paragraphs" class="words">
          <template v-for="(p, i) in paragraphs" :key="i">
            <!-- between paragraphs: the pull quote leads into the paragraph it comes from, else three stars -->
            <blockquote v-if="i > 0 && i === quoteAt">{{ quote }}</blockquote>
            <p v-else-if="i > 0" class="stars" aria-hidden="true">✦ ✦ ✦</p>
            <p>{{ p }}</p>
          </template>
        </div>
        <div v-else class="words md"><MarkdownText :text="text" /></div>
        <nav v-if="!compact && (around.previous || around.next)" class="turn">
          <button v-if="around.previous" @click="emit('open', around.previous)">‹ {{ shortDate(around.previous.date) }} <span>{{ around.previous.title }}</span></button>
          <span v-else />
          <button v-if="around.next" @click="emit('open', around.next)">{{ shortDate(around.next.date) }} <span>{{ around.next.title }}</span> ›</button>
        </nav>
      </div>
      <ol v-if="!compact && around.around.length > 1" class="thread" aria-label="Nearby days">
        <li v-for="p in around.around" :key="p.file" :class="{ here: p.here }" :title="p.title" @click="emit('open', p)">
          <span class="dot" /><span class="label">{{ shortDate(p.date) || '·' }}</span>
        </li>
      </ol>
    </template>

    <!-- C: the handwritten night journal -->
    <template v-else>
      <div class="notebook-sheet">
        <span class="tape top" aria-hidden="true" /><span class="tape bottom" aria-hidden="true" />
        <svg v-if="parts" class="stamp" viewBox="0 0 80 80" aria-hidden="true">
          <defs><path id="stamp-arc" d="M 14 40 A 26 26 0 0 1 66 40" /></defs>
          <circle cx="40" cy="40" r="36" /><circle cx="40" cy="40" r="31" class="inner" />
          <text class="around"><textPath href="#stamp-arc" startOffset="50%" text-anchor="middle">{{ parts.weekday.toUpperCase() }}</textPath></text>
          <text x="40" y="47" text-anchor="middle" class="mid">{{ parts.month }} {{ parts.day }}</text>
          <text x="40" y="62" text-anchor="middle" class="star">★</text>
        </svg>
        <h1>{{ page.title }}</h1>
        <svg class="flourish" viewBox="0 0 200 12" preserveAspectRatio="none" aria-hidden="true"><path d="M2 8 C 50 2, 110 11, 198 4" /></svg>
        <p v-if="problem" class="problem">{{ problem }}</p>
        <div v-else-if="paragraphs" class="words">
          <p v-for="(p, i) in paragraphs" :key="i">
            <svg class="mark" viewBox="0 0 24 24" aria-hidden="true">
              <path v-if="markOf(i) === 'sprout'" d="M12 21 V10 M12 12 C 7 12, 5 8, 6 5 C 10 5, 12 8, 12 12 M12 10 C 15 10, 18 8, 18 4 C 14 4, 12 7, 12 10" />
              <path v-else-if="markOf(i) === 'star'" d="M12 3 L14.4 9.2 L21 9.6 L15.9 13.8 L17.6 20.3 L12 16.7 L6.4 20.3 L8.1 13.8 L3 9.6 L9.6 9.2 Z" />
              <path v-else d="M12 20 C 4 14, 3 8, 7.5 6 C 10 5, 12 7.5, 12 9 C 12 7.5, 14 5, 16.5 6 C 21 8, 20 14, 12 20 Z" />
            </svg>
            {{ p }}
          </p>
        </div>
        <div v-else class="words md"><MarkdownText :text="text" /></div>
        <p v-if="by" class="signed">– {{ by }}</p>
      </div>
    </template>
  </article>
</template>

<style scoped>
.diary-page {
  min-height: 0;
  height: 100%;
  overflow-y: auto;
  color: var(--text);
}
.problem {
  font-size: 13px;
  color: var(--warn);
}
h1 {
  margin: 0;
  font-weight: 400;
}

/* ---------- A: a letter under starlight ---------- */
.letter {
  --serif: 'Iowan Old Style', 'Palatino Linotype', Palatino, 'Book Antiqua', Georgia, serif;
  display: flex;
  justify-content: center;
  padding: 10px 6px 18px;
}
.letter .sheet {
  width: 100%;
  max-width: 34em;
  height: fit-content;
  padding: 34px 40px 26px;
  border: 1px solid color-mix(in srgb, var(--line) 70%, transparent);
  border-radius: 14px;
  background: radial-gradient(120% 90% at 50% 0%, color-mix(in srgb, var(--accent) 11%, transparent), transparent 70%), color-mix(in srgb, var(--text) 3%, transparent);
  box-shadow: 0 0 40px color-mix(in srgb, var(--accent) 7%, transparent);
  font-family: var(--serif);
  font-size: 17px;
  line-height: 1.75;
}
.letter .when {
  margin: 0;
  text-align: center;
  font-family: inherit;
  font-size: 11px;
  letter-spacing: 0.26em;
  text-transform: uppercase;
  color: color-mix(in srgb, var(--accent) 70%, var(--text));
}
.letter .rule {
  display: block;
  width: 120px;
  height: 10px;
  margin: 12px auto 18px;
}
.letter .rule polyline {
  fill: none;
  stroke: color-mix(in srgb, var(--accent) 55%, transparent);
  stroke-width: 0.5;
}
.letter .rule circle {
  fill: var(--text);
}
.letter h1 {
  margin-bottom: 24px;
  text-align: center;
  font-family: var(--serif);
  font-size: 1.65em;
  line-height: 1.25;
}
.letter .words p {
  margin: 0 0 1.15em;
}
.letter .words p.first::first-letter {
  float: left;
  margin: 0.04em 0.1em 0 0;
  font-size: 3.2em;
  line-height: 0.86;
  color: var(--accent);
}
.letter .ornament {
  margin: 22px 0 10px;
  text-align: center;
  font-size: 14px;
  color: var(--accent);
}
.letter .foot {
  margin: 0;
  text-align: center;
  font-size: 11px;
  letter-spacing: 0.2em;
  color: var(--text-dim);
}
.letter.compact .sheet {
  max-width: none;
  padding: 24px 26px 20px;
  font-size: 15.5px;
}

/* ---------- B: the constellation journal ---------- */
.journal {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 26px;
  padding: 6px 4px 14px;
}
.journal .main {
  max-width: 44em;
  min-width: 0;
}
.journal header {
  display: flex;
  align-items: center;
  gap: 20px;
}
.journal .date-block {
  display: flex;
  align-items: center;
  gap: 10px;
}
.journal .day {
  font-size: 52px;
  font-weight: 200;
  line-height: 1;
}
.journal .stack {
  display: grid;
  font-size: 12px;
  line-height: 1.5;
}
.journal .stack b {
  font-weight: 500;
  letter-spacing: 0.14em;
  color: var(--accent);
}
.journal .stack i {
  font-style: normal;
  color: var(--text-dim);
}
.journal .emblem {
  width: 64px;
  height: 46px;
}
.journal .emblem polyline {
  fill: none;
  stroke: color-mix(in srgb, var(--accent) 60%, transparent);
  stroke-width: 0.5;
}
.journal .emblem circle {
  fill: var(--text);
}
.journal h1 {
  margin: 12px 0 22px;
  font-family: 'Iowan Old Style', 'Palatino Linotype', Palatino, Georgia, serif;
  font-style: italic;
  font-size: 28px;
  line-height: 1.25;
}
.journal .words {
  font-size: 15.5px;
  line-height: 1.8;
}
.journal .words p {
  margin: 0;
}
.journal .words .stars {
  margin: 14px 0;
  text-align: center;
  font-size: 8px;
  letter-spacing: 0.9em;
  color: color-mix(in srgb, var(--accent) 70%, transparent);
}
.journal blockquote {
  margin: 20px 0;
  padding: 2px 0 2px 18px;
  border-left: 2px solid var(--accent);
  box-shadow: -6px 0 14px -8px var(--accent);
  font-family: 'Iowan Old Style', 'Palatino Linotype', Palatino, Georgia, serif;
  font-style: italic;
  font-size: 1.22em;
  line-height: 1.5;
  color: color-mix(in srgb, var(--accent) 80%, var(--text));
}
.journal .turn {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  margin-top: 28px;
}
.journal .turn button {
  max-width: 48%;
  padding: 7px 14px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 12.5px;
  cursor: pointer;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.journal .turn button:hover {
  border-color: var(--accent);
}
.journal .turn span {
  color: var(--text-dim);
}
.journal .thread {
  position: sticky;
  top: 8px;
  display: grid;
  gap: 34px;
  height: fit-content;
  margin: 8px 6px 0 0;
  padding: 6px 0;
  list-style: none;
  border-left: 1px solid color-mix(in srgb, var(--line) 80%, transparent);
}
.journal .thread li {
  position: relative;
  padding-left: 14px;
  font-size: 11px;
  color: var(--text-dim);
  cursor: pointer;
}
.journal .thread .dot {
  position: absolute;
  left: -4px;
  top: 4px;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: color-mix(in srgb, var(--text) 55%, transparent);
}
.journal .thread li.here {
  color: var(--text);
}
.journal .thread li.here .dot {
  left: -6px;
  top: 2px;
  width: 11px;
  height: 11px;
  background: var(--accent);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 25%, transparent), 0 0 12px var(--accent);
}
.journal .thread li:hover {
  color: var(--text);
}
.journal.compact {
  grid-template-columns: 1fr;
}

/* ---------- C: the handwritten night journal ---------- */
.notebook {
  --row: 34px;
  --hand: 'Segoe Script', 'Lucida Handwriting', 'Brush Script MT', cursive;
  --pen: 'Ink Free', 'Segoe Print', 'Comic Sans MS', cursive;
  padding: 14px 10px 20px 4px;
}
.notebook .notebook-sheet {
  position: relative;
  max-width: 40em;
  margin: 0 auto;
  padding: 26px 34px 30px 74px;
  border: 1px solid color-mix(in srgb, var(--line) 80%, transparent);
  border-radius: 10px;
  /* the ruled lines, and the margin line near the left edge */
  background:
    linear-gradient(to right, transparent 54px, color-mix(in srgb, var(--accent) 45%, transparent) 54px, color-mix(in srgb, var(--accent) 45%, transparent) 55px, transparent 55px),
    repeating-linear-gradient(to bottom, transparent 0, transparent calc(var(--row) - 1px), color-mix(in srgb, var(--text) 9%, transparent) calc(var(--row) - 1px), color-mix(in srgb, var(--text) 9%, transparent) var(--row)),
    color-mix(in srgb, var(--text) 2.5%, transparent);
  background-position: 0 0, 0 calc(var(--row) * 4 + 6px), 0 0;
}
.notebook .tape {
  position: absolute;
  width: 54px;
  height: 18px;
  background: color-mix(in srgb, var(--text) 14%, transparent);
  border: 1px solid color-mix(in srgb, var(--text) 10%, transparent);
}
.notebook .tape.top {
  left: -16px;
  top: 8px;
  transform: rotate(-38deg);
}
.notebook .tape.bottom {
  right: -16px;
  bottom: 10px;
  transform: rotate(-38deg);
}
.notebook .stamp {
  position: absolute;
  right: 22px;
  top: 18px;
  width: 76px;
  height: 76px;
  transform: rotate(8deg);
  color: color-mix(in srgb, var(--accent) 85%, var(--text));
}
.notebook .stamp circle {
  fill: none;
  stroke: currentColor;
  stroke-width: 1.2;
}
.notebook .stamp circle.inner {
  stroke-width: 0.5;
}
.notebook .stamp text {
  fill: currentColor;
  font-family: var(--pen);
}
.notebook .stamp .around {
  font-size: 8px;
  letter-spacing: 1.5px;
}
.notebook .stamp .mid {
  font-size: 13px;
  font-weight: 700;
}
.notebook .stamp .star {
  font-size: 9px;
}
.notebook h1 {
  max-width: calc(100% - 96px);
  min-height: calc(var(--row) * 2);
  font-family: var(--hand);
  font-size: 25px;
  line-height: var(--row);
}
.notebook .flourish {
  display: block;
  width: min(62%, 260px);
  height: 12px;
  margin: 2px 0 calc(var(--row) - 14px);
}
.notebook .flourish path {
  fill: none;
  stroke: color-mix(in srgb, var(--text) 75%, transparent);
  stroke-width: 1.2;
  stroke-linecap: round;
}
.notebook .words {
  font-family: var(--pen);
  font-size: 18.5px;
  line-height: var(--row);
}
.notebook .words p {
  position: relative;
  margin: 0 0 var(--row);
}
.notebook .mark {
  position: absolute;
  left: -58px;
  top: 5px;
  width: 22px;
  height: 22px;
  fill: none;
  stroke: color-mix(in srgb, var(--text) 80%, transparent);
  stroke-width: 1.2;
  stroke-linecap: round;
  stroke-linejoin: round;
}
.notebook .signed {
  margin: 0;
  text-align: right;
  font-family: var(--hand);
  font-size: 20px;
  line-height: var(--row);
}
/* an entry with markdown in it keeps its own spacing */
.words.md {
  line-height: 1.7;
}
</style>
