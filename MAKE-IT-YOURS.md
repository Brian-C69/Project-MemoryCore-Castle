# Make It Yours

MemoryCore Castle is MIT licensed: copy it, change it, rename it, share it. This guide shows the usual changes. Castle is a [Tauri 2](https://tauri.app/) app: a Vue 3 page (`src/`) on top of a small Rust core (`src-tauri/`).

## 1. A theme of your own

The easiest way: open **THEME > Create theme**, give it a name and a picture, and Castle makes it for you.

By hand, a theme is a folder with a `theme.json` and a wallpaper:

```
%APPDATA%\com.memorycore.castle\themes\sakura-night\
  theme.json
  wallpaper.png
```

```json
{
  "id": "sakura-night",
  "name": "Sakura Night",
  "description": "soft pink blossoms under a dark blue night",
  "wallpaper": "wallpaper.png",
  "wallpaperPosition": "center",
  "widgetStyle": "glass",
  "colors": {
    "--background": "#0b0a1f",
    "--text": "#fdf0f6",
    "--text-dim": "#c6a9bb",
    "--accent": "#ff9ccf",
    "--line": "rgba(255, 170, 210, 0.34)",
    "--panel-background": "rgba(24, 14, 40, 0.62)",
    "--widget-background": "rgba(30, 18, 48, 0.55)",
    "--bar-background": "rgba(12, 8, 26, 0.72)",
    "--hover": "rgba(255, 156, 207, 0.14)",
    "--glass-blur": "12px"
  },
  "panel": { "left": "1.6%", "top": "3%", "bottom": "3%", "width": "31%" },
  "widgets": {
    "clock": { "right": "21%", "top": "4%", "size": "l" },
    "gauges": { "right": "2%", "top": "4%" },
    "usage": { "right": "31%", "bottom": "calc(4% + 150px)", "width": "33%" },
    "today": { "right": "2%", "top": "22%", "width": "28%" },
    "projects": { "right": "2%", "top": "calc(22% + 190px)", "width": "28%" },
    "diary": { "right": "2%", "top": "calc(22% + 395px)", "width": "28%" },
    "growth": { "right": "2%", "bottom": "4%", "width": "28%" },
    "backup": { "right": "31%", "bottom": "4%", "width": "16%" }
  }
}
```

**Sweep** (HOME's cleaner) has three looks: `ring`, `bar` and `broom`. A theme may choose the one it starts with, and give Sweep a place like any widget:

```json
"sweep": { "look": "broom" },
"widgets": { "sweep": { "right": "31%", "top": "22%" } }
```

The look a member picks in Sweep's gear always wins over the theme's. A theme that names no place for Sweep still shows it, at its own place.

**Widget skin** - a theme may dress HOME's cards and the companion panel with `"widgetSkin"`, on top of the glass look: `prism-lines` (no boxes, a prismatic light line under each widget), `carved-wood` (wooden frames and amber glass), `sunset-glass` (amber-to-rose glass with a gold rim), `bioluminescent` (navy glass with a glowing cyan edge) or `ripple-glass` (emerald glass with ripple rings). A name Castle does not know is ignored.

```json
"widgetStyle": "glass",
"widgetSkin": "bioluminescent"
```

**A living wallpaper (a built-in theme)** - put a short seamless loop beside the wallpaper and name it: `"motionVideo": "wallpaper-loop.mp4"` (MP4 or WebM, muted, about 4 MB). It plays over the still picture while Moving wallpapers is on, and is never loaded while it is off. A loop whose first and last frames are the wallpaper itself never shows its join: The Star Shore's, The Sunset Shore's and The Emerald River's were made that way with Seedance image-to-video, 8 seconds at 1080p.

**Living water** - a theme may instead give its wallpaper gentle motion drawn by code on HOME with a `motion` block. Every position is a fraction of the picture (`[across, down]`, each 0 to 1), so the effect stays on the water at any window size. Two kinds:

```json
"motion": { "kind": "surf", "color": "#5fd8ff", "count": 170,
            "crests": [[[0.0, 0.70], [0.5, 0.63], [0.95, 0.54]], [[0.0, 0.84], [0.48, 0.66]]] }
```

`surf`: glowing points ride each crest (a line of points along a breaking wave) and a brighter wash runs along it now and then.

```json
"motion": { "kind": "river", "color": "#e8f7c8", "count": 70, "flow": [0.05, 1],
            "water": [[0.52, 0.21], [0.17, 1.0], [0.92, 1.0], [0.56, 0.21]],
            "ripples": [[0.56, 0.60]] }
```

`river`: light streaks drift across the `water` outline in the `flow` direction, longer and faster nearer the bottom, and ripple rings spread from each `ripples` point and now and then elsewhere on the water.

```json
"motion": { "kind": "shore", "line": [[0.42, 1.0], [0.68, 0.78], [1.0, 0.5]], "toward": [1, 0.6], "reach": 0.025 }
```

`shore`: the foam at the waterline (`line`, where the wet sand meets the dry) washes up the sand along `toward` by up to `reach` and draws back on a slow sea rhythm.

```json
"motion": { "kind": "twinkle", "area": [[0, 0], [1, 0], [1, 0.7], [0, 0.7]], "count": 40 }
```

`twinkle`: Castle finds the brightest stars inside `area` in the picture itself and lets them flare in turn.

`color` and `count` are optional (at most 240 points for surf, 160 streaks for a river, 80 stars). A block Castle cannot read is ignored. Motion is off until a member turns it on in **Settings > Rooms > Moving wallpapers**, and it rests while HOME is hidden or when the system asks for reduced motion.

Rules the loader checks (`src/themes/loader.js`): an `id` and a `name`; colours only as `--variables` in `colors`; widget places only `left`, `top`, `right`, `bottom`, `width`, `height` (and `size` for the clock: `s`, `m` or `l`). A widget your theme leaves out is not shown (the USAGE card is only on HOME when the theme gives it a place). A broken theme is listed with its reason and never shown. Press **Reload themes** after editing.

**Native controls follow the theme too.** Scrollbars, checkboxes, radio buttons, sliders and the dropdown lists take their look from the theme, so none of them shows the plain Windows grey. Their defaults read your theme's own colours; to change them, set any of these in `colors` like any other variable:

| variable | what it changes | default |
|---|---|---|
| `--scrollbar-thumb` | the draggable part of every scrollbar | `var(--line)` |
| `--scrollbar-track` | the groove behind it | `transparent` |
| `--scrollbar-width` | `thin`, `auto` (wider) or `none` (hidden) | `thin` |
| `--control-accent` | checkboxes, radio buttons and sliders | `var(--accent)` |
| `--color-scheme` | `dark` or `light`: native pickers, spin buttons, selection | `dark` |

A light theme should set `--color-scheme` to `light`. When you add a screen of your own, use `GlassSelect` for a dropdown (not a plain `<select>`) and leave scrollbars to these variables; a test fails the build if a page does otherwise.

To ship a theme **inside** your build, put its folder in `src/themes/<id>/` instead - built-in themes are found automatically.

### A start-up screen for your theme

A theme can dress the start-up screen too. Add a `loader` block to `theme.json` (every part is optional):

```json
"loader": {
  "image": "loader.webp",
  "brand": "SAKURA NIGHT",
  "quips": ["Counting petals.", "Brewing blossom tea."],
  "quipsMode": "add",
  "accent": "#ff9ccf",
  "position": "bottom"
}
```

| key | what it does |
|---|---|
| `image` | a 16:9 picture in the theme's own folder (PNG, JPEG or WebP; keep it under about 3 MB so it can be remembered) |
| `brand` | the small title above the funny line (up to 40 characters) |
| `quips` | your own funny lines (up to 60, each up to 120 characters) |
| `quipsMode` | `add` (default) puts them with Castle's own; `replace` shows only yours |
| `accent` | the bar and title colour: `#hex`, `rgb()` or `hsl()` (default: the theme's `--accent`) |
| `position` | where the words sit: `bottom` (default), `center` or `top` |

The start-up screen is on screen before any theme is chosen, so it shows a theme's design from the **second start** after the theme was picked: Castle remembers the design of the theme in use. A bad value (an odd colour, an unknown `position`, a missing picture) only means Castle's own choice shows; it never makes the theme unusable. Aim the words so they stay readable: with `position` `bottom` keep the bottom third of the picture calm, with `top` the top third, with `center` the middle.

### A start-up screen of your own

Castle opens on a start-up screen: a picture, a funny line, a bar and the name of what it is loading. Two files in `public/loader/` change it, and nothing else needs touching:

- `loader.webp` - the picture. 16:9 (for example 1920x1080), with the subject in the top and middle, because the words sit over the bottom third. Without it the screen shows its own night-sky gradient.
- `quips.json` - more funny lines: an array of strings (up to 60, each up to 120 characters), added to the built-in ones in `src/loader/quips.js`.

The steps it names come from `App.vue` (`loaderBegin` / `loaderEnd`); a room that takes a while to prepare can report its own the same way.

## 2. A room of your own

1. Make the view: `src/views/MyRoomView.vue`.
2. Add it to the tab list in `src/App.vue` (`allTabs`), to the `BUILT` set, and render it:
   ```vue
   <MyRoomView v-if="active === 'myroom'" />
   ```
3. If it needs the computer (files, programs), add a Rust command in `src-tauri/src/` (see `memory.rs` for reading the memory folder safely with `files::fenced`), register it in `src-tauri/src/lib.rs` (`generate_handler!`), and call it from `src/api/castle.js`.
4. A room that needs a MemoryCore feature can hide until it is installed: give its tab `needs: '<feature>'` and add the check to `src-tauri/src/memory.rs` (`features`).

### Load at start-up, not on arrival

The start-up screen exists so that **a room is already full when the member opens it**. A room that
loads its data or pictures when it is opened makes them wait, so move that work to start-up. The
pattern (STUDIO is the working example: `src/composables/useStudio.js`, `src-tauri/src/thumbs.rs`):

1. **Keep the room's data outside the room.** Put it in a small module (`src/composables/useMyRoom.js`)
   as shared `ref`s, so it lives for the whole run instead of being thrown away each time the room
   closes (rooms are mounted and unmounted as the member moves between tabs).
2. **Give the module a `prepareMyRoom(progress)`** that reads everything the room needs, and calls
   `progress(done, total)` as it goes. Read quietly again from the room's `onMounted`, so what is
   already loaded shows first and anything new swaps in.
3. **Report it to the start-up screen** in `src/App.vue`, in the list of start-up pieces:

   ```js
   piece('myroom', 'Preparing my room', 'my-folder/', () =>
     prepareMyRoom((done, total) => loaderBegin('myroom', 'Reading my pictures', `${done} / ${total}`)))
   ```

   Raise `loaderExpect(n)` by one for the new piece. The status line shows the label and the file or
   place being read; calling `loaderBegin` again with the same id just updates it.
4. **Do the heavy part in the core, in turns.** A Rust command that reads or converts many things
   should be `async` with `spawn_blocking`, do a few at a time and return how many are left, so the
   page can ask again until none are (see `studio_thumbs`).
5. **Never wait forever.** Start-up waits for each piece only up to a limit (`App.vue`), and the screen
   itself leaves after `MAX_MS` (`src/loader/loader.js`). A slow or failing piece lets Castle open and
   keeps going in the background.
6. **Big pictures get small copies.** A list or gallery shows small JPEGs made once and kept in the
   settings folder; the real picture opens only when the member asks for it.

A room that follows this opens at once, shows what it has, and checks for news quietly.

## 2b. A world of your own

Members can create worlds from the WORLD room (see the user guide). To ship a 3D world **inside** your build:

1. Make `src/worlds/<id>/world.json`:
   ```json
   {
     "slug": "my-world", "title": "My World", "line": "One sentence for its card",
     "layout": "centrepiece", "camera": { "default": "orbit", "scale": 3 }, "glow": "#8fe6ff",
     "pieces": [
       { "file": "01-base.glb", "role": "base", "size": 40 },
       { "file": "02-heart.glb", "role": "companion", "size": 9 },
       { "file": "03-islet.glb", "role": "islet", "size": 8, "orbit": { "radius": 40, "height": 2, "lap": 260, "phase": 0 } }
     ]
   }
   ```
   Roles: `base` (the great isle, its top at the world's floor), `companion` (stands at the heart and glows), `islet` (circles on its `orbit`), `isle` (an archipelago island standing in for one of the built-in isles: `"isle": "heart" | "north" | "east" | "west" | "south"`) and `prop` (at a fixed `at`). `size` is the piece's largest side in metres - pieces are sized by their bounding box, so any export scale works.
2. Put your GLB exports, named by number, in a folder and run `npm run world:lighten -- <id> <folder>` (default folder: `Project Resources/Design/<id>/hd`). The prepared pieces land in `public/worlds/<id>/`; add a `preview.jpg` there for the card. To tune sizes, orbits and the camera by eye, run `npm run vite` and open `http://localhost:5173/?world=3d/<id>&camera=orbit` (dev builds only). A world's `camera` may also set its own `"orbit": { "radius", "height" }` and `"wide": { "radius", "height" }` in metres.
3. The card turns *ready* when every piece is there. Anything you add here ships with your build, so use models you have the rights to share (Tripo's free plan publishes under CC BY 4.0; a paid plan owns its output).

## 2c. A workflow of your own

The easiest way: **STUDIO > ＋ New workflow**, or describe one for your companion. By hand, a workflow is a folder with one `workflow.json`:

```
%APPDATA%\com.memorycore.castle\workflows\two-cats\
  workflow.json
```

```json
{
  "schema": 1,
  "name": "Two cats",
  "icon": "heart",
  "power": "image",
  "description": "My two cats, together, anywhere",
  "blocks": [
    { "id": "a", "type": "character", "label": "First cat" },
    { "id": "b", "type": "character", "label": "Second cat" },
    { "id": "scene", "type": "text", "label": "Where they are" },
    { "id": "style", "type": "chips", "label": "Style", "options": ["Anime", "Painterly", "Photo"] },
    { "id": "make", "type": "create", "prompt": "{{a.name}} and {{b.name}}: {{scene}}. Style: {{style}}.",
      "references": ["a", "b"], "size": "1536x1024", "quality": "medium" }
  ]
}
```

**Blocks:**
- `picture` is an upload.
- `character` is a picture and a name. `{{id.name}}` puts the name in the prompt.
- `text` is a text box. `{{id}}` puts its words in.
- `chips` is one choice from `options`. `{{id}}` puts the choice in.
- `describe` reads a `picture` block (`from`) and writes a prompt into a `text` block (`into`).
- Exactly one `create`, last: the `prompt`, up to five `references` to follow (character or picture blocks), a `size` (`1024x1024`, `1024x1536` or `1536x1024`) and a `quality` (`low`, `medium` or `high`).

**Icons:** `picture`, `people`, `person`, `poster`, `star`, `heart`, `spark`, `brush`.

The whole format is written out in [`docs/workflow-schema.json`](docs/workflow-schema.json). Unknown keys are kept and ignored. A broken file stays on the rail with its reason, and a workflow never runs code: Castle only reads the words.

## 3. Teach the companion something

- Everything your companion is told about Castle lives in `src-tauri/resources/castle-block.md` (written into the memory's `CLAUDE.md` and `AGENTS.md` between `castle` markers).
- The STORY rules: `src-tauri/resources/story-rules.md`. The chat room rules: `src-tauri/resources/room-rules.md`. What the companion reads in private to choose what it shares there: `src-tauri/resources/room-shelf-rules.md`.
  Its `{{MANNER}}` line is where the companion's emotion engine goes (only its active traits' one-line manner, by audience, written by `roomturn::manner_section`); the proposals' rules are `src-tauri/resources/emotion-rules.md`.
- The drawing styles: `src-tauri/resources/styles/*.md`.
- The media helpers: `src-tauri/resources/tools/castle-*.mjs` (run `node src-tauri/resources/tools/test-helpers.mjs` after a change).

## 4. Rename it

- `package.json` (`name`), `src-tauri/Cargo.toml` (`name`), `src-tauri/tauri.conf.json` (`productName`, `identifier`, window `title`).
- Changing the `identifier` gives your build its own settings folder and its own keys in Credential Manager (`src-tauri/src/key.rs`, `SERVICE`).
- The icon: `npx tauri icon your-icon.png`.
- The Credits wall in Settings > About: the names and what each person built are hand-written in `src/contributors/people.js` (`FOUNDER`, `PEOPLE`, `THANKS`); the merged counts and avatars in `src/contributors/` come from `npm run contributors` (also run by `npm run bump`). Change `REPO` in `scripts/release-tools.mjs` and `OWNER` in `scripts/contributors-tools.mjs` to count your own repository.

## 5. Build your installer

```bash
npm install
npm run tauri build
```

The installer lands in `src-tauri/target/release/bundle/nsis/`.

**Updates:** only official releases update themselves. A build you make yourself never does, so it can never overwrite your changes. To bring in new Castle features, pull the upstream repository with git and merge them into your copy:

```bash
git remote add upstream https://github.com/Kiyoraka/Project-MemoryCore-Castle.git
git pull upstream main
```

## 6. Tests

```bash
npm test                      # web tests (vitest)
cd src-tauri && cargo test    # core tests (uses the fixture memories in src-tauri/tests/fixtures)
node src-tauri/resources/tools/test-helpers.mjs   # media helpers, offline
```

Two fixture memories keep Castle honest: `memory-fresh` (a new MemoryCore) and `memory-existing` (a member who already set things up). Check new features against both.
