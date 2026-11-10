<script setup>
// A small drawing of one STORY stage, for the THEME room's showroom: the shape of the look
// (where the scene, the words and the choices sit), not a running story.
defineProps({
  look: { type: String, default: 'classic' }, // classic | cinema | book | messenger | jrpg
  // the scene behind it: the main theme's wallpaper
  scene: { type: String, default: '' },
})
</script>

<template>
  <div class="mini" :class="look" :style="scene ? { '--scene': `url(&quot;${scene}&quot;)` } : null">
    <template v-if="look === 'classic'">
      <div class="box">
        <span class="name">Nova</span>
        <p>The lake is so still tonight… it almost feels like it is listening to us.</p>
        <span class="more">▸</span>
      </div>
    </template>

    <template v-else-if="look === 'cinema'">
      <span class="band top" /><span class="band bottom" />
      <p class="subtitle">The lake is so still tonight… it almost feels like it is listening.</p>
      <ul class="side"><li>Cross now</li><li>Find another way</li></ul>
    </template>

    <template v-else-if="look === 'book'">
      <div class="pages">
        <div class="left" />
        <div class="right">
          <h4>Chapter 3</h4>
          <p>The path leads to a narrow stone bridge, hung between two islands. The wind howls below, and the moonlight paints the void.</p>
        </div>
      </div>
    </template>

    <template v-else-if="look === 'messenger'">
      <div class="thread">
        <p class="them"><b>Nova</b>The lake is so still tonight.</p>
        <p class="them">It almost feels like it is listening.</p>
        <p class="me">Then let us be quiet with it.</p>
        <div class="quick"><span>Cross now</span><span>Find another way</span></div>
      </div>
    </template>

    <template v-else>
      <div class="frame">
        <div class="face" />
        <div class="words">
          <span class="name">Nova</span>
          <p>The stars are calling us.</p>
          <ul><li>▸ Cross now</li><li>Find another way</li></ul>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.mini {
  position: relative;
  height: 100%;
  min-height: 0;
  overflow: hidden;
  border-radius: 8px;
  background: var(--scene, linear-gradient(160deg, #1a2a6c, #0b1130 60%, #05070f)) center / cover no-repeat;
  color: var(--text);
  font-size: 12px;
  line-height: 1.5;
}
p {
  margin: 0;
}
.name {
  font-size: 11px;
  font-weight: 600;
  color: var(--accent);
}

/* Classic VN: the scene fills the frame, a glass box with a name plate at the bottom */
.classic .box {
  position: absolute;
  left: 6%;
  right: 6%;
  bottom: 7%;
  padding: 10px 14px 12px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: color-mix(in srgb, #04081c 78%, transparent);
  backdrop-filter: blur(6px);
}
.classic .name {
  display: block;
  margin-bottom: 2px;
}
.classic .more {
  position: absolute;
  right: 10px;
  bottom: 6px;
  color: var(--accent);
}

/* Cinematic: letterboxed, the words as subtitles, choices at the side */
.cinema .band {
  position: absolute;
  left: 0;
  right: 0;
  height: 13%;
  background: #000;
}
.cinema .band.top {
  top: 0;
}
.cinema .band.bottom {
  bottom: 0;
}
.cinema .subtitle {
  position: absolute;
  left: 10%;
  right: 10%;
  bottom: 17%;
  text-align: center;
  text-shadow: 0 1px 4px #000, 0 0 10px #000;
}
.cinema .side {
  position: absolute;
  right: 5%;
  top: 22%;
  display: grid;
  gap: 6px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.cinema .side li {
  padding: 3px 10px;
  border: 1px solid color-mix(in srgb, var(--text) 45%, transparent);
  border-radius: 999px;
  background: rgba(0, 0, 0, 0.45);
  font-size: 10.5px;
}

/* Storybook: an open book, the scene on the left page, the story on the right */
.book {
  display: grid;
  place-items: center;
  background: radial-gradient(circle at 50% 40%, #1b2248, #070a1a);
}
.book .pages {
  display: grid;
  grid-template-columns: 1fr 1fr;
  width: 86%;
  height: 72%;
  border-radius: 6px;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.6);
  overflow: hidden;
}
.book .left {
  background: var(--scene, linear-gradient(160deg, #1a2a6c, #0b1130)) center / cover no-repeat;
  box-shadow: inset -14px 0 18px -12px rgba(0, 0, 0, 0.7);
}
.book .right {
  padding: 12px 14px;
  background: #efe6d2;
  color: #2b2418;
  font-family: 'Palatino Linotype', Palatino, Georgia, serif;
  box-shadow: inset 14px 0 18px -12px rgba(0, 0, 0, 0.35);
}
.book h4 {
  margin: 0 0 6px;
  font-size: 12px;
  font-weight: 600;
}

/* Messenger: the story as a chat thread, choices as quick replies */
.messenger {
  background: color-mix(in srgb, #04081c 88%, var(--accent));
}
.messenger .thread {
  display: flex;
  flex-direction: column;
  gap: 6px;
  height: 100%;
  padding: 14px;
  box-sizing: border-box;
}
.messenger p {
  max-width: 78%;
  padding: 6px 10px;
  border-radius: 12px;
}
.messenger .them {
  align-self: flex-start;
  background: color-mix(in srgb, var(--text) 12%, transparent);
}
.messenger .them b {
  display: block;
  font-size: 10px;
  color: var(--accent);
}
.messenger .me {
  align-self: flex-end;
  background: var(--accent);
  color: #07142e;
}
.messenger .quick {
  display: flex;
  gap: 6px;
  margin-top: auto;
}
.messenger .quick span {
  padding: 3px 10px;
  border: 1px solid var(--accent);
  border-radius: 999px;
  font-size: 10.5px;
  color: var(--accent);
}

/* JRPG window: a framed window with the speaker's face, choices under the words */
.jrpg .frame {
  position: absolute;
  left: 5%;
  right: 5%;
  bottom: 7%;
  display: grid;
  grid-template-columns: 68px 1fr;
  gap: 10px;
  padding: 10px;
  border: 2px solid color-mix(in srgb, var(--text) 80%, transparent);
  border-radius: 6px;
  outline: 1px solid var(--accent);
  outline-offset: 2px;
  background: linear-gradient(180deg, rgba(14, 26, 84, 0.92), rgba(5, 10, 40, 0.94));
}
.jrpg .face {
  aspect-ratio: 1;
  border: 1px solid color-mix(in srgb, var(--text) 60%, transparent);
  border-radius: 4px;
  background:
    radial-gradient(circle at 50% 38%, color-mix(in srgb, var(--accent) 75%, #fff) 0 22%, transparent 23%),
    radial-gradient(ellipse at 50% 105%, color-mix(in srgb, var(--accent) 60%, #fff) 0 46%, transparent 47%),
    color-mix(in srgb, var(--accent) 22%, #050a28);
}
.jrpg ul {
  margin: 4px 0 0;
  padding: 0;
  list-style: none;
  font-size: 10.5px;
  color: var(--text-dim);
}
.jrpg li:first-child {
  color: var(--text);
}
</style>
