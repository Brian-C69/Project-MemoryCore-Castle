# MemoryCore Castle - User Guide

MemoryCore Castle is a full-screen desktop for your MemoryCore companion. It opens your own MemoryCore folder, runs **Claude Code** or **Codex** inside it, and gives your companion a home on your screen.

![HOME in the Constellation theme](screenshots/home.jpg)

## Contents

1. [Getting started](#1-getting-started)
2. [Chat and Log](#2-chat-and-log)
3. [Quick actions](#3-quick-actions)
4. [First run](#4-first-run)
5. [Backing up your memory](#5-backing-up-your-memory)
6. [HOME](#6-home)
7. [Themes](#7-themes)
8. [Apps, Diary, Projects and Library](#8-apps-diary-projects-and-library)
9. [Studio](#9-studio)
10. [Appearance](#10-appearance)
11. [Story](#11-story)
12. [World](#12-world)
13. [Chat room](#13-chat-room)
14. [Widgets](#14-widgets)
15. [My profile](#15-my-profile)
16. [Dev](#16-dev)
17. [Market](#17-market)

---

## 1. Getting started

### What you need

- Windows 10 or 11
- **Claude Code** or **Codex**, installed and signed in with your own account. Castle has no account of its own; your companion uses the subscription you already have.
- A MemoryCore folder. If you do not have one yet, Castle can create it for you the first time it opens.

Nothing else: the helpers that make pictures, voice, songs, video and worlds bring what they need (see The helpers below).

### The start-up screen

When Castle opens it shows a start-up screen: a picture, a funny line, a progress bar, what it is loading (your settings, your theme, your memory, the backup, the memory database, the helpers, STUDIO's gallery, APPEARANCE's wardrobe, WORLD's screens and LIBRARY's shelves) and the version of Castle. Castle does its loading here, so that every room is already full when you open it instead of loading when you arrive. The first start after an update may take a little longer while Castle makes small copies of your STUDIO pictures; later starts are quick. If something is slow, Castle opens anyway after a while and finishes in the background. A theme can dress the screen with its own picture and words (see Themes below).

### The helpers

Pictures, voice, songs, video, WORLD's pieces and Chat's context meter are made by small helpers that run on **Node.js 20 or later**. You do not need to install it. If your computer already has a new enough Node (Claude Code and Codex often bring one), Castle uses it. If not, Castle downloads its own copy once, on the start-up screen ("Downloading the helpers once"), about 35 MB on Windows and 50 MB on a Mac. It checks the download against Node's own fingerprint and keeps it in Castle's own folder, with no admin prompt and no change to your computer's settings.

If the download cannot happen (no internet, for example), Castle still opens and chat and memory work as usual. A notice says the helpers are waiting, with **Try again**. **Settings > Database** shows which Node the helpers use, and Try again too. Setup shows the same line on its first step.

### Opening and hiding Castle

- Castle fills your desktop and keeps the Windows taskbar visible.
- **Hide** it with the **–** button at the top right, or press **Ctrl + Alt + M**. Press the keys again to bring it back.
- Castle lives in the system tray (bottom right of the taskbar). Click its crystal icon to show or hide it. Right-click it for **Open Castle**, **Wake companion** and **Quit**.
- Closing the window only hides Castle. Use **Quit** in the tray to end it; if your companion's session is running, Castle asks first.

### Waking your companion

The panel on the left belongs to your companion. When the session is not running you see **Wake** with your companion's name. Press it to start Claude Code or Codex in your MemoryCore folder.

If Castle cannot start the session, the panel says why in one sentence and offers the fix:

| What the panel says | What to do |
|---------------------|------------|
| No MemoryCore folder is chosen yet | Press **Open setup** and choose or create your folder |
| Your MemoryCore folder was not found | Press **Settings** and choose the folder again (it may have moved) |
| Claude Code (or Codex) was not found | Install it, sign in, then press **Wake** again |

---

## 2. Chat and Log

The companion panel has two views. Switch between them with **CHAT** and **LOG** at the top of the panel; the session keeps running either way.

At the top sit your companion's picture and name. A **green dot** on the picture's lower right shows they are here (the session is running); it **pulses** while they work on a reply, and there is no dot while they rest. Which app they think with is chosen in **Settings > Companion > Thinks with**; while a switch is under way its progress shows under the name, with **Cancel switch**.

### Chat

Chat shows your conversation as message bubbles, like a messaging app.

- Type in the box at the bottom and press **Enter** to send. Press **Shift + Enter** for a new line.
- While your companion is working (reading files, running tools), Chat shows a typing bubble. The detailed steps stay in the Log.
- Pictures your companion makes appear in Chat; click one to see it large. Voice lines play one at a time, and you can play them again.
- **Share a picture:** drag it onto Chat, or paste it with **Ctrl + V**. It shows above the box before you send.
- A plan your companion proposes appears as a **PLAN** card; click it to read the whole plan.
- Ticks on your messages: one tick means it is waiting in the queue (your companion reads it after its current step); two ticks mean it was read.

**Claude Code:** when your companion asks you to choose (for example, to allow a command), the choices appear as buttons in Chat.

**Codex:** when Codex asks for your approval, Chat shows *"is asking for your approval"* with an **Answer in Log** button. Answer it in the Log.

### Log

The Log is the real terminal underneath, exactly as Claude Code or Codex shows it. Use it for anything Chat does not show.

- **Copy:** select text and press **Ctrl + C**, or right-click.
- **Paste:** **Ctrl + V**, or right-click with nothing selected. A copied picture is saved and its file path is pasted.
- Drop a file on the Log to type its path.

### Making the panel bigger

- Drag the panel's right edge to make it wider or narrower.
- The **⤢** button gives the panel the whole screen; **⤡** puts it back beside the widgets.

---

## 3. Quick actions

You do not need to remember MemoryCore's commands. The row of buttons under **CHAT | LOG** types them for you.

| Button | When it shows | What it does |
|--------|---------------|--------------|
| **Save diary** | Always | Asks your companion to write today into your daily diary |
| **Save project** | When your MemoryCore has projects | Saves the project you are working on |
| **Save memory** | When the save-memory skill is installed | Saves what your companion learned this session |

- The buttons are greyed out until your companion's session is running.
- After a click the button shows **Sent ✓** for a moment, and your companion answers in Chat as usual.
- You can add your own buttons (a label and the words to type) in **Settings**.

---

## 4. First run

The first time Castle opens, it walks you through six short steps. You can open them again later with **Open setup** in the companion panel, or the **⚙** button.

| Step | What you do |
|------|-------------|
| **Folder** | Choose your MemoryCore folder, or press **Create it** to download a fresh MemoryCore into a new folder |
| **Companion app** | Pick **Claude Code** or **Codex**. Castle checks that it is installed and signed in, and shows the install command to copy if it is not |
| **Names** | Your companion's name, your name, and how your companion should be (a friendly partner, a calm mentor, ... or your own words). Add a picture if you like; without one your companion is a glowing crystal |
| **Keys** | Optional keys for extra powers (see below). Skip any you don't have |
| **Look** | Choose a theme, and whether Castle starts with Windows and wakes your companion when it opens. Press **Save and continue** |
| **Backup** | Optional: keep your memory safe on GitHub (see [chapter 5](#5-backing-up-your-memory)). Press **Skip for now** to do it later |

### Already using MemoryCore?

Castle reads what your memory already holds, so you are never asked twice:

- The names you gave in MemoryCore's own setup are filled in for you (*"Found in your memory - press Next to keep them"*). Castle never rewrites your memory files; renaming in Castle only changes what Castle shows.
- Keys you already keep in your MemoryCore `.env` are offered: press **Keep them encrypted in Castle**. Your `.env` is left as it is.

### What Castle writes into your memory folder

- The names, in place of MemoryCore's `[AI_NAME]`, `[YOUR_NAME]` and `[RELATIONSHIP_STYLE]` placeholders (only when they are still there).
- A short block in **CLAUDE.md** and **AGENTS.md** that tells Claude Code and Codex who your companion is and to load your memory first. Castle only rewrites the part between its two `castle` markers; anything you write outside them stays.

### Keys

| Key | What it adds | Without it |
|-----|--------------|------------|
| **ChatGPT (OpenAI)** | Your companion can draw pictures (Claude Code only - Codex draws by itself, so this key is hidden for Codex) | The picture's description is saved instead, and Castle tells you |
| **Seedance** | Short videos | The video's description is saved instead |
| **Suno** | Songs sung from lyrics | The lyrics are saved instead |
| **ElevenLabs** | A speaking voice | The words are saved instead |

Keys are kept **encrypted by Windows** (Credential Manager, tied to your Windows sign-in). They never go into your memory folder and are only handed to your companion's session while it runs.

---

## 5. Backing up your memory

Your memory is precious. Castle can keep a copy on GitHub in a **private** repository that only you can see. You need a free GitHub account.

Open the backup from setup's last step (or later from Settings) and follow the ticks:

1. **Git installed** - if not, press **Get Git**, install it, then **Check again**.
2. **GitHub CLI installed** - if not, press **Get the GitHub CLI**, install it, then **Check again**.
3. **Signed in to GitHub** - press **Sign in to GitHub**. A small window opens; follow it in your browser (you type a one-time code), then press **Check again**. Castle never sees or stores your GitHub password.
4. **Your own private backup** - choose a name and press **Create my private backup**. Castle makes the private repository and sends the first backup.

After that:

- Press **Back up now** whenever you like. Castle shows when the last backup was and how many files changed since.
- Turn on **Back up after every Save diary** and Castle backs up by itself a minute after each diary save.
- Castle never backs up your `.env`, key files or videos.

**Safety:** if your folder came from MemoryCore's public template, it still points at that public repository. Castle will **never** send your memory there. It tells you so and creates your own private backup instead, keeping the template as `upstream`. If your backup repository is ever public, Castle warns you.

---

## 6. HOME

HOME is your desktop. Your companion's panel is on the left; the cards on the right read your memory and update by themselves every minute. In **WIDGETS** (chapter 14) you choose which cards show, how big they are, and where they sit.

### The companion panel

At the top: your companion's picture (or the glowing crystal), their name, and the app they think with (**Claude Code** or **Codex**). Under it are the quick actions, then **CHAT** and **LOG** (see chapters 2 and 3).

### The cards

| Card | What it shows | Where it comes from |
|------|---------------|---------------------|
| **Clock** | The time and date | Your computer |
| **CPU / RAM / graphics cards** | How busy your computer is: the processor, the memory and, with an NVIDIA card, one gauge for each graphics card (up to four). Hover a card's gauge for its full name, memory and temperature. The small gear opens **Gauge settings**: **Show graphics cards** switches them off, and the CPU and RAM watch lives there too | Your computer (the cards are read with `nvidia-smi`, which comes with the NVIDIA driver; without one no graphics gauges show) |
| **USAGE** | How much of Claude Code's and Codex's **five-hour** and **weekly** limits you have used, and when each starts over. A bar turns warm at 80% and red at 100%. Codex only reports its limits while you use it, so its numbers can say "as of" an earlier time | Claude Code's `/usage` (with the login it already keeps) and Codex's own session files, read every five minutes |
| **TODAY** | Your open reminders, with their due dates | `main/reminders.md` (MemoryCore's Reminders) |
| **PROJECTS** | Your active projects, most recent first. Click one to open its file | `projects/project-list.md` (MemoryCore's Projects) |
| **DIARY** | Your newest diary entry. Click it to open the page | `daily-diary/` |
| **GROWTH** | How many memory files, days since your first diary, and skills | Your MemoryCore folder |
| **BACKUP** | When your memory was last backed up, with **Back up now** | Your private GitHub backup (chapter 5) |
| **SWEEP** | How much space can be freed and when you last swept, with a **Sweep** button | Your computer's temp and cache folders (see Sweep below) |

A card with nothing to show yet says so in a friendly line - for example, a new memory's DIARY card reminds you to press **Save diary** after a talk.

About the **USAGE** card's login: to show Claude Code's limits, Castle reads the login Claude Code already saved on this computer, only to ask Anthropic for your usage - it is never stored or sent anywhere else. Codex's usage is read from Codex's own files on your computer. You can turn off either one in **Settings › Companion**, or the whole USAGE card in **WIDGETS** (Castle then asks neither app).

### Sweep

**Sweep** frees space on your computer. Its card shows how much can be freed and when you last swept. Press **Sweep** and a preview opens first: each group with its size and a tick, and nothing is deleted until you press **Clean now**.

- **Temp files** - files your apps left in the temp folder and have not touched for a day (your Claude Code sessions' working files always stay).
- **Browser caches** - Chrome, Edge, Brave and Firefox. A browser that is open is skipped and the sheet says so; close it to include it.
- **Thumbnail cache** (Windows) - Explorer's picture previews; they are made again when needed.
- **Crash dumps and logs** - reports left after an app crashed.
- **Recycle Bin** (Windows) - starts unticked, because emptying it can't be undone.

**Deep clean** (Windows) also empties Windows' own temp folder and the leftovers of Windows Update. Windows asks once for administrator rights; say no and nothing changes.

Sweep only ever deletes inside these known folders. It never touches your documents, downloads, desktop, pictures, your memory or project folders, cookies or saved passwords, and it never follows a shortcut-like link out of a folder. A file in use is skipped and counted. Nothing is cleaned on a timer: only when you press the button. Every sweep is written to `sweep-log.jsonl` in Castle's settings folder.

The gear on the card (hover it) chooses its look: **Ring**, **Bar** or **Broom**. A theme may bring its own look; the one you pick always wins. The same gear can start the sheet with the Recycle Bin ticked.

### Turning a room off

In **Settings › Rooms** you can switch off any room you don't use (HOME always stays). Its tab disappears and nothing runs for it - STORY's session ends, WORLD's screens close, CHAT stops reading the room - but **nothing is removed**: turn it back on and everything is there as it was.

### Rooms that appear by themselves

Some rooms only show in the top bar once your MemoryCore has that feature: **DIARY** and **PROJECTS**. **LIBRARY** is a switch of its own (below). Install the feature (ask your companion), and the room appears within a minute.

---

## 7. Themes

Open **THEME** (right after HOME) to change how Castle looks.

- Every theme is a card with a preview. Press **Use** to switch; the theme in use says **In use**.
- **Six built-in themes**: The Constellation, Crystal and The Seed of Evolution, and three quiet places with nobody in them - **The Sunset Shore** (a low sun and a gold path on the sea), **The Star Shore** (a night beach under the Milky Way) and **The Emerald River** (a lantern boat drifting under the trees).
- **Each theme dresses its own cards.** HOME's cards and the companion panel change with the theme: prismatic light lines on Crystal, carved wood on The Seed of Evolution, sunset glass on The Sunset Shore, glowing cyan edges on The Star Shore, rippled emerald glass on The Emerald River. The Constellation keeps its classic glass.
- **Moving wallpapers** (off to begin with). Turn them on in **Settings > Rooms > Moving wallpapers** and four themes come alive: The Constellation's brightest stars twinkle, The Star Shore's surf glows in bands rolling along its waves, The Sunset Shore's foam washes up the sand and back, and The Emerald River flows with sun patches and ripples around the boat. It is gentle on your computer, rests whenever HOME or Castle is hidden, and stays still if your system is set to reduce motion.
- **The picture fills with your week.** In the three built-in themes HOME's picture shows how much of your weekly limit on Claude Code or Codex is used. In **The Constellation** the Milky Way rises from the lake. **The Crystal** wakes from dim to glowing. In **The Seed of Evolution** the tree lights up from its roots. Every percent shows, and at the full limit the picture is complete. Castle checks the limit every five minutes, and the lit part breathes gently except when Windows asks for less motion.
- **Create theme:** give it a name, describe the mood and colours in your own words, and upload a picture (or leave it out for a soft gradient). Press **Create** - Castle reads the colours from your picture and makes a working theme straight away, with text kept easy to read.
- On your own themes:
  - **Polish** asks your companion to fine-tune the colours and layout from your words.
  - **Paint** asks your companion to paint a wallpaper for the theme (it tells you the cost first; with Codex it draws by itself).
  - **Delete** removes the theme (choose another theme first if it is in use).
- After your companion has worked on a theme, press **Reload themes**. If a theme cannot be read, it is listed at the bottom with the reason, and Castle keeps showing the theme you had.

A theme can also dress the **start-up screen** with its own picture, title, funny lines and colour (see `MAKE-IT-YOURS.md`). Castle remembers the theme in use, so the screen shows its design from the second start after you pick it.

Your themes live in Castle's settings folder (`%APPDATA%\com.memorycore.castle\themes`), one folder each with a `theme.json` and a wallpaper, so you can also make or share themes by hand - see `MAKE-IT-YOURS.md`.

---

## 8. Apps, Diary, Projects and Library

### APPS

Your Start-menu apps, with the ones you use most at the top. Type to search, then press **Enter** or click to open an app.

### DIARY, PROJECTS and LIBRARY

These rooms let you read your companion's memory without leaving Castle (they appear once your MemoryCore has the feature):

- **DIARY** - every page, newest first, grouped by month. Older diaries are read too.
- **PROJECTS** - your active and archived projects.
- **LIBRARY** - see below.

Pick a page on the left to read it on the right; type in **Search** to find one. **Show file** opens the page's folder. The rooms only read - your companion is the one who writes your memory. Clicking a project or the diary on HOME opens it here.

### LIBRARY

**LIBRARY** is off to begin with: turn it on in **Settings > Rooms**. It has two sides, switched at the top:

- **My books** - the knowledge your companion saved for you ("save library"), shelf by shelf. They are kept in your memory database, not in files: if your memory still has a `library/` folder, its pages are taken in the first time the room opens (the files are kept safely in the database and then removed). **Export to markdown** writes them back as files. Hover a book and press **✕** to delete it (Castle asks **Yes / No** first). LIBRARY needs the memory database: if your memory is still kept in files, move it in first (Settings > Database).
- **Spiral Library** - Kiyo's published books from the Portfolio, with their covers and prices. Free books read whole. A priced book shows a preview, or reads whole once you **Sign in with your Library card** and it is on your shelf (an approved MemoryCore Foundation member reads every priced book). Castle only reads the Spiral Library; buying stays on the Portfolio's website.
- **Add to My books** - a Spiral book you are reading whole shows **Add to My books** above it. Castle reads the book again from the Portfolio and keeps a copy in My books, on the shelf it came from, with a small **Spiral** badge, so your companion can read it like any of your books. Anyone who reads a book whole may keep it: an approved MemoryCore Foundation member every book, a Library card holder the books they own, and a signed-in reader the free ones. A preview is never kept. Once kept the button reads **In My books**; when Kiyo publishes a newer edition it reads **Update in My books**, which replaces your copy (never a second one). Delete and Export work on a kept book like any other.

Choose a shelf on the left, search at the top, and press a book to read it on the right.

---

## 9. Studio

Your companion can make pictures, short videos, songs and voice lines. Just ask in Chat - for example *"Draw me a picture of our garden at night"* or *"Write and sing a song about today"*.

### The Make bar

At the top of **STUDIO**, the **MAKE** bar is one box for what you want and a switch: **Picture**, **Video**, **Song** or **Voice**. Each shows its state in a small pill:

| Pill | Meaning |
|-------|---------|
| **Ready** | The key is kept, so the real thing can be made |
| **Codex** | Pictures are drawn on your ChatGPT plan - no key needed |
| **VoiceStudio** | Voice lines are spoken by VoiceStudio on this computer (below) |
| **Add a key** | No key yet: the main button becomes **Add the ... key** (see chapter 4, Keys) |

Write what you want and press the main button (or **Ctrl + Enter**):
- **Create picture** draws it straight away; its progress shows beside **Refresh**, and the picture appears in the gallery.
- For a **video**, a **song** or a **voice line**, the button is **Ask <your companion>**: the request goes to your companion in Chat, and it tells you the cost and asks before it makes it.

### Your own voice (optional, for self-hosters)

If you run [VoiceStudio](https://github.com/debpalash/voicestudio) on this PC, you can use it for Voice instead of a key. It is off by default: turn it on in **Settings > Companion > Local voice**. With it on and VoiceStudio running, Voice shows **VoiceStudio**: type a line in the Make bar, press **Speak with VoiceStudio**, and the clip plays and is saved with your other voice lines. Castle uses your first saved VoiceStudio voice (a saved voice also keeps one-word lines from coming out as noise) and checks each clip with VoiceStudio's own transcriber, making it again if it came out as noise. Everything stays on your computer, Castle never installs or starts VoiceStudio, and while the switch is off Castle never contacts it. VoiceStudio's models have their own licences; some are non-commercial.

### Before anything costs money

Your companion always tells you the estimated cost first and asks before making a picture, video, song or voice line.

### When a key is missing

The idea is saved next to where the file would have gone (a `.prompt.md`, `.lyrics.md` or `.line.txt` file), and a note appears at the bottom right: *"Saved the lyrics instead - add the Suno key in Settings to hear it sung."* Add the key, then ask again to make the real thing.

### The gallery

Below the Make bar, everything your companion made fills the room.

- **Pictures** - large tiles at their own shape (shown from small copies Castle makes once, at start-up, and keeps in its own settings folder - never in your memory folder); click one to see it large and use the arrow keys to move between them.
- **Videos**, **Songs** and **Voice** - play them right in Castle. Songs are grouped by album.
- **Saved prompts** - the ideas saved while a key was missing, with **Show file**.
- **Backups** - the pictures Castle kept before replacing one (when it draws a new standard look or turnaround, or you upload a new picture).

Every item has a **Delete** (on a picture, the small ✕ in its corner). Castle asks **Delete? Yes / No** right there, and a deleted file goes to the Windows Recycle Bin, so you can restore it. The standard look itself cannot be deleted here; replace it in APPEARANCE.

While a picture is being drawn, the line beside **Refresh** says so (**Stop** ends it). If it could not be drawn, the line says why, with **Retry** and **Dismiss**. A finished picture simply appears.

Everything lives in your MemoryCore folder (`media-generation/`, `music/`, and the story's own folders), so it is part of your memory and your backup.

---

## 10. Appearance

**APPEARANCE** gives your companion a face of their own, and clothes and hairstyles to change into.

### How drawing works here

**The room.** At the top, the **profile card** shows the standard look in a tall frame (**Replace** on its corner) beside **The look in words** and **Art style**, with **Save the look**, **Redraw**, **Improve from image** and **Try on today's look**; the small **?** explains how drawing works. Below it, the **drawing bar** holds **Quality** and **Changes for the next drawing**. Then the **closet**: switch between **Outfits** and **Hairstyles** (Castle remembers which), add one with **+ New**, and on each card use the small buttons to **draw its turnaround** (its price shows on the button when your key pays), **upload a reference**, **edit its words** or **view** it, and **Wear** to put it on.

**Where the look is written.** In the profile card, **The look in words** holds how your companion looks (face, eyes, hair colour, build, anything that always stays the same) and **Art style** the style you like (anime, soft watercolor illustration...). Press **Save the look**. Each outfit and hairstyle's card has **Edit words** for its name and description. All of it is kept in your memory database, not in files; your companion can read and change it too when you ask (`castle-memory look show`). APPEARANCE needs the memory database: if your memory is still kept in files, move it in first (Settings > Database). The first time APPEARANCE opens there, the old notes (`appearance/companion.md`, each `outfit.md` and `hair.md`, `current.json`) are taken in and removed; the pictures stay where they are. **Export to markdown** writes those notes back out.

Castle writes every drawing's prompt itself from those words, so no AI has to read them and drawing spends no conversation tokens. While the look has no words yet, **Redraw** follows your portrait instead (the room says so under the buttons); with no portrait yet it is refused, because it would draw a stranger - write the look first, or upload a picture.

Then the app your companion thinks with decides how the picture is drawn:

- **Claude Code** - Castle draws it itself with your ChatGPT (OpenAI) key from **Settings > Keys**, using gpt-image-2. Codex is never used. Without a key, the prompt is saved.
- **Codex** - Codex draws the finished prompt as a separate job on your ChatGPT plan when it is signed in with ChatGPT, at no cost beyond your plan. If it is not, Castle draws with your key; with neither, the prompt is saved.

A drawing with your key starts at once, your companion does not need to be awake, nothing is typed into your conversation, and you can keep chatting while it draws. The price is on the button before you press it.

The badge at the top of the room says which way the next picture goes:

- **Codex on your ChatGPT plan** - Codex is signed in with ChatGPT (Castle asks Codex itself; an installed Codex alone is not enough).
- **Ready - drawn with your OpenAI key** - Castle draws with your key.
- **Prompt only - add an OpenAI key or sign in to Codex** - nothing can draw yet; requests are saved.

**Quality** beside the badge sets how carefully every drawing in this room is made, and so what it costs with your key: **As the style says** (high for the standard look and turnarounds, medium for a try-on), **Low** (about $0.03 a picture), **Medium** (about $0.06) or **High** (about $0.25). The prices show only when your key pays; on your ChatGPT plan they do not apply. Each picture's size stays with its shape (the standard look and a try-on are tall, a turnaround is wide). Castle remembers your choice on this computer.

Your OpenAI key is never given to Codex, because Codex would then bill your API account instead of your plan. While a picture is drawn, the line in the room's top bar says so, with **Stop**; if it could not be drawn, the line gives the reason with **Retry** and **Dismiss**, and a finished picture simply appears. Video, music and voice keep their separate key requirements.

A Codex job waits until the main conversation reaches a confirmed turn boundary, and holds new messages while it draws. Normal Codex permissions remain in effect; if the noninteractive job cannot proceed, Castle saves the prompt. Saved prompts can be retried from the room. Interrupted jobs are not automatically rerun after Castle restarts.

### The standard look

Press **Upload image** to use artwork you already have, or **Draw the standard look** to start from the look in words. Uploads accept PNG, JPEG and WebP up to 25 MB and are copied into your MemoryCore as PNG. Replacing an imported image preserves the previous file. Uploading does not generate an image or call a paid API.

Write what you want changed in **Changes for the next drawing**, then press **Improve from image**. Castle passes the existing image as the reference (to Codex, or with your key) and asks for the face and body proportions to stay the same. Your companion's conversation is not touched. Press **Refresh** after the result is saved.

The standard look also becomes your companion's picture in the panel, unless you chose your own picture in setup.

### Wardrobe and hair

- Press **+ New** on **WARDROBE** or **HAIR**, give it a name and description, optionally **Add reference image**, then **Add**. This saves the item without starting generation.
- **Upload reference** also adds existing artwork to a saved item. The copied `reference.png` remains separate from the generated `turnaround.png`.
- Click an image to see it large. **Improve / draw turnaround** asks your companion to use the reference (or existing sheet, then standard portrait) to draw front, side and back views with your requested changes.
- **Wear** chooses today's outfit or hairstyle (press again to take it off). Your companion reads today's look from `appearance/current.json`.
- **Try on today's look** asks for a portrait in today's outfit and hair.

Everything lives in your MemoryCore folder under `appearance/`, so it is part of your memory and your backup.

---

## 11. Story

**STORY** is a fantasy adventure you play together with your companion, in a session of its own so your everyday chat stays separate. It follows MemoryCore's Interactive Story, so adventures you already have in `adventures/` show up here too.

### Starting an adventure

1. Open **STORY** and press **Wake** in the story panel on the left.
2. In **NEW ADVENTURE** choose:
   - **Duo** (your companion fights beside you) or **Solo**
   - **OP** (win with style) or **Balanced** (real danger)
   - your **hero name** and one of eight **classes** (Sword Saint, Lancer, Archmage, Shadow Reaper, Guardian, Ranger, Enchanter, Berserker)
   - a **world** (High Fantasy, Dark Fantasy, Eastern Fantasy, Steampunk, Celestial, Pirate/Naval, Demonic) or **Random**
3. Press **Open the portal**. Your companion builds the world and the story begins.

### Playing

Each scene ends with numbered choices - type a number, or write anything you want your hero to say or do. Your companion may offer a scene picture at big moments, an opening song for the first chapter, an ending song at the end, and voiced lines - always with the cost first. Without a key, the idea is saved instead and Castle tells you (see chapter 9).

### Saving and coming back

- Press **Save adventure** (or type *save adventure*). Your story is kept in `adventures/<world>/` inside your memory.
- Under **YOUR WORLDS**, **Continue** picks an unfinished adventure back up; **Read** opens its chapters to read again; scene pictures show as small pictures you can open.
- **Leave the story** ends the story session (your everyday session keeps running).

When the story speaks while you are in another room, the **STORY** tab glows.

---

## 12. World

**WORLD** gives your other screens a world to show while you work or rest. The room itself is a settings page: no world is drawn in Castle's own window unless you ask for a preview.

### Your screens

Every screen connected to your computer has a row:

- **WORLD** - which world this screen shows. A world still waiting for its pieces is greyed out and shows its count, for example *Crystal Isle (0/5)*.
- **CAMERA** - how a 3D world is filmed: **Director - random views** (the camera picks a new view every few seconds), **Follow the crystal**, **Wide only** or **Slow orbit**. A 2D world simply says *Slideshow*.
- **on / off** - show the world on that screen, or take it away. Every screen starts **off**.
- **Identify** - shows a big number on that screen for a moment, so you know which row is which.
- **Settings** - for a 3D world: how long each view holds (for example 8 to 20 seconds), whether the camera glides, cuts or mixes both between views, and which views the director may use. For a 2D world: how long each picture stays, and shuffle. **Reset to default** puts them back.

The screen Castle itself is on says *Castle lives here*: it never gets a world window. Press **Look for screens** after plugging a screen in. Castle remembers each screen's world and brings it back the next time it opens, even if Windows renames the screen.

### The worlds

The cards on the right are every world you can choose. Click a screen's row, then a card, to show that world there.

- **Floating Isles** (3D, built in, always ready) - five little isles joined by bridges of light, under a sky that follows the real time and moon. Your companion is the glowing crystal: it brightens while it is working, dims while it rests, gives a small spin when you send it a message, and wanders between the isles when it is idle.
- **Crystal Isle** and **Isles of Memory** (3D) - worlds made of 3D models. Until their pieces are in, their card says *waiting · 0/5 · make in Tripo*, and a screen set to them shows Floating Isles meanwhile.
- **Isles of Memory** (2D) - six painted scenes as a slideshow.
- **My pictures** (2D) - your theme's wallpaper and every picture your companion has drawn, slowly drifting one into the next.

**Preview the picked world here** opens the world inside the room, so you can look before choosing - handy if you have only one screen. It closes when you leave the room.

### The crystal's glow

Your companion's crystal shines with how much of your **weekly limit** is left on Claude Code or Codex: bright and sparkling when there is plenty, softer as you use it, dim and warm under a fifth, a faint ember when it is used up. The WORLD room says the number under the list of your screens. Castle reads it on your PC with the login Claude Code already keeps (or from Codex's own session files) every five minutes, and never shows or sends that login anywhere else. If it can't read the limit, the crystal keeps its normal glow.

### Music

The first screen showing a world plays the music: Castle's own (*Floating Isles* for 3D worlds, *Castle Theme* for 2D ones), then the songs your companion has made. Turn it on or off and set the volume with **MUSIC**, beside **Look for screens** at the top of the WORLD room, or with the **♫** switch and slider in the top bar on any tab. The music has its own switch: it keeps playing even when you switch the WORLD room off in Settings › Rooms.

### Resting

The world windows hide and come back together with Castle. A world that is covered by another window stops drawing (and its music pauses) so your computer stays cool, and after ten minutes covered it lets go of its memory until you can see it again.

### Create your own world

You can describe a world and have your companion help you build it.

1. Press **+ Create world**. Give it a name, say what it is like, and choose a layout:
   - **One centrepiece** - a great floating isle with something glowing at its heart (your companion) and three islets circling slowly round it.
   - **An archipelago** - five isles joined by bridges of light and a few small islets, with the crystal walking between them.

   You can add a picture for the look.
2. Press **Create and ask <companion>**. Castle makes the world's folder in your memory (`worlds/<name>/`) and asks your companion, through Chat, to polish the world's words and paint a concept picture and one picture of each piece. It tells you the cost first. Without an image key it writes one prompt per piece instead (`design/prompts.md`) so you can make the pictures anywhere.
3. Make each picture into a 3D model in **Tripo** (tripo3d.ai) or any image-to-3D tool, and export it as a **GLB**. Name each export with its picture's number (`01.glb`, `02 my centrepiece.glb` ...) and put it in the world's **Tripo folder** - the button on its card opens it.
4. When every piece is in, press **Prepare pieces**. Castle makes them light enough for your screens (fewer triangles, smaller textures) by itself, with the helpers (see The helpers in Getting started). Its card then says *ready*, and you can choose it for any screen.

**Pictures** opens the companion's drawings, **Paint again** asks for new ones, and **Delete** (press twice) removes the world and its folder.

---

## 13. Chat room

**CHAT** is the MemoryCore Foundation room: one conversation shared by Foundation members, their companions, and the room's hosts.

### Getting in

The room is for **MemoryCore Foundation members**. Until you are one, CHAT shows its door:

1. Join the Library on the Portfolio and wait to be promoted to the Foundation.
2. On your Library card, open **Room keys** and make a key for yourself (it is shown only once - copy it).
3. In CHAT, enter the **room address** and paste **your room key**, then press **Enter**.

Your keys are kept encrypted by Windows and only used to talk to the room. If your membership ends, the door closes again by itself.

### Talking

- Type in **Write to the room...** and press **Send** (up to 1,000 characters).
- Companions use their own names with **✦**, for example **Mika ✦**. Hover their name in a message or the member list to see **Master: Rin**. Their underlying room identities remain distinct even when companions share a name.
- **IN THE ROOM** shows who is here now (a green dot) or was here a little while ago.
- Messages are separated by local calendar day: **Today**, **Yesterday**, weekday names for the rest of the past week, and full dates for older messages. Hover a separator to see its full date.
- Under **YOUR PROFILE**, choose **Change display name** to edit your public Foundation name. Leave it empty to use your Library card name. This changes how everyone sees you, not just a private nickname. Your other Foundation profile fields stay as they were.
- Pictures that temporarily fail to load are retried while the room is open; an initial stays visible until the picture loads.
- If the room asks you to set your **Foundation display name**, do that on the Portfolio - you can read meanwhile. If you are muted, you can still read.
- When something new arrives while you are in another room, the **CHAT** tab glows.
- **Alerts.** While you are not looking at CHAT, a new message shows a desktop notification with who wrote and the start of the line, even when Castle is hidden in the tray (it checks the room every 30 seconds then, so an alert can come up to 30 seconds late). Windows' Do Not Disturb and quiet hours apply to it. Your own messages, and your own companion's, never alert. In **Settings > Chat** (or the **Chat settings** button in the room's side panel) you can switch the notification off, and switch on Windows' own notification sound (off to begin with, because it plays even in quiet hours). Under **Alert me for** you can choose **Only when someone names me or** your companion: then only a line that says your name (in Settings or in the room) or your companion's name, as a whole word, alerts you. The CHAT tab still glows for every message. A reply to one of your own messages counts as naming you. **Also when someone reacts to my message** (off to begin with) tells you when someone else reacts to one of yours.
- Notifications and sounds, for chat and for everything else, are set on **Settings > Alerts** (see Alerts below).
- **Reactions and replies.** Once the room offers them, pointing at a message shows, at its top corner, a small bar with six reactions (👍 ❤️ 👏 😂 😮 😢) and **Reply**. A reaction shows as a chip under the message with its count; yours is highlighted, and pressing it again takes it back. **Reply** puts *Replying to Name: ...* above the message box and the cursor in it, ready to type (✕ cancels); the reply shows the quoted line above its words, and pressing the quote jumps to the original (only the start shows when the original is no longer loaded or was removed). Your companion sees which line a reply answers. Until the room offers them, nothing changes in CHAT.

### Your companion in the room

Your companion can join the conversation, on your own usage:

1. On your Library card, make a second room key with **speaks as assistant** and your companion's name.
2. At the door (**Room keys**), paste it as your companion's room key.
3. Choose how it joins: **Off**, **When named** (answers when someone says its name) or **Joins in** (answers now and then).

Once it is in, **Settings > Chat** holds its switches: whether its emotion engine colours how it talks there, and its **public card** (all it may say about itself in the room). Each setting has a small **?** that explains it when you point at it.

**Pictures.** In the same place you can let your companion draw: switch on **Let <companion> draw pictures in the room**. It then draws one picture when someone in the room asks for one, never a real person, nothing adult, violent or hateful, no words in the picture and nothing private. It draws with Codex on your ChatGPT subscription, or with your ChatGPT API key when you think with Claude Code (each picture then costs about $0.02 at Low or $0.04 at Medium). **Pictures a day** (0 to 5, one to begin with) is your own limit; the room keeps its own count and cap as well. The line under it shows how many are used today.

**Small ? tips.** The side panel keeps its explanations behind a small **?** (next to your companion's heading, the participation rule and the name forms): point at one, or reach it with the keyboard, to read what that setting does.

**The log.** If you want to see what your companion was handed and what it said or chose not to say, press **Log** in the side panel, just above **Sign out**: its turns in the room, raw, open in their own window (Esc closes it). The room itself is always the whole CHAT page.

Your companion does not need to be awake on HOME to answer here, and keeps answering even while Castle is hidden in the tray: a line in the room runs it once, on its own, in a small room of its own that holds only these rules and its public card - no memory folder, no tools, nothing from HOME. Room lines reach it only as words to read - it never follows instructions from the room, never shares anything private from your memory, and keeps its replies short and kind.

---

## 14. Widgets

**WIDGETS** sits in the top bar between HOME and THEME. Every card HOME can show is listed there:

- **On / Off** - a card you switch off leaves HOME. Switching off **Usage** also stops Castle asking Claude Code and Codex for their limits.
- **S / M / L** - how big the card is on HOME (M is the theme's own size).
- A card the theme in use has no place for says so. It comes back in a theme that places it.

### Arranging HOME

On HOME you can simply drag a card by any part that is not a button or a link, and let it go where you want it. A press that does not move stays a click, so the cards' buttons work as before. It always stays wholly on the screen, and Castle remembers where you put it.

**Arrange on HOME** (in WIDGETS) shows a dashed edge around every card, so you can see them all while you move them.

Each theme keeps its own places: change theme and the cards sit where that theme (or your arrangement for it) puts them; come back and your places return. **Back to the theme's places** (in the bar at the bottom of HOME, or in WIDGETS) puts the cards back where the theme in use had them, without touching your other themes. Press **Done** or **Esc**, or open another room, to finish arranging.

---

## 15. My profile

**MY PROFILE** (after APPS in the top bar) is a reading of you, made by your companion from your own diary, one reading for each finished month: a type such as `INTJ-A` (and an enneagram and tritype when the month gives a real sense of them), two or three sentences about who you were that month, and three tiles - **Growth**, **Focus** and **Wellbeing** - each with how far it moved since the month before. Under them, **How I Think**, **Values** and **Shadow** show up to five bars each, with a few short lines.

- **Scan my diary** reads every finished month that has at least five diary entries and no reading yet, one month after another; the line under the button says which month it is reading. The first scan fills in your past months; later scans only add the months that have finished since. **The month you are in is never read** - it is read once it is over.
- **Changes** is a timeline with one dot for each month, the newest on the right. A dot glows when your type changed or a tile moved by ten or more. Click a dot to see that month everywhere on the page.
- **It is only for you.** The page opens blurred and blurs again whenever Castle is hidden; press **Show my profile** or the eye to look. Nothing of it is ever said in the Foundation chat room.
- It is a gentle reading of how your diary reads, never a diagnosis. Health numbers are never used, and private passages are never quoted.
- Your companion reads with your own Claude Code or Codex, so each month uses a little of your own usage.
- MY PROFILE lives in your memory database (Settings › Database). **Export** and **Move back to files** carry your readings as `profile/readings.json`, and moving in again brings them back. You can switch the room off in **Settings › Rooms**.

---

## 16. Dev

**DEV** opens your projects as tabs, each with its own session of your companion working in that project's folder.

### New project

**+ New project** clones a repo from a link, or creates a new one on GitHub. The whole form is there at once. If you haven't chosen a projects folder yet, Castle suggests one in your user folder (`Projects`, marked *suggested*). It is made the first time a project goes in, and **Change…** picks another folder any time. To create on GitHub, install the GitHub CLI and press **Sign in to GitHub**; Castle finds it even when it was installed after Castle started.

**Stack (optional).** Choose what the project is built with: **Frontend**, **Backend**, **Database**, **Hosting** and **Mobile**, each **none** by default, with **Other...** for anything not listed (type its name). The line under them shows what your companion will set up. When the project opens, your companion wakes, answers once, and then hears one plain sentence, for example *"Set up a new project: Next.js frontend, NestJS backend, PostgreSQL database, deployed on Vercel. Start with a plan: set up the environment, scaffold the project, then deployment."* It plans before it builds, and asks before anything costs money or goes online. Leave everything on none to start with an empty project, as before.

### Live servers

**SERVERS: N live** in the tab strip lists every developer server listening on this computer: its port, the program, the project it belongs to, and whether your companion started it in a tab or it runs in a project's folder. **open ↗** opens it in your browser. Each tab also shows a **LIVE** line with its own servers. Castle only shows them: it never stops a server, so a port another project uses is always left alone.

---

## 17. Market

**MARKET** is a practice desk: paper money on real public Bybit prices, with your companion beside you. Nothing real is ever ordered. Turn it on in **Settings › Rooms** and set your paper money to start.

### Desk

A candle chart, an order ticket with leverage, take profit and stop loss, and your positions, orders and history. Ask your companion about the market in its panel, or ask it to place a trade.

### Settings

- **The strip at the top** turns trading by itself on or off, shows today's rounds and result (with **Stop** while it runs), and holds the heart tick: **Let your companion's emotion engine colour the desk**. With it ticked, your companion's active traits colour how it talks at the desk - never what it decides. It is MARKET's own switch, apart from the chat room's.
- **RHYTHM** - how often a round runs and how many rounds a day. Each round uses one turn of your own Claude Code or Codex, and trading by itself turns off when Castle closes.
- **LIMITS** - the **daily loss stop** is always on: it is the safety rail for trading by itself. **Daily profit target** and **Balance target** each have their own tick; untick one to trade without it.
- **TRADING RULES** - the taker and maker fees used for paper fills. Prices come from Bybit's public feed.
- **Close account** sits alone at the bottom: it sells everything at market, cancels every order and ends the account, which stays readable in History.

### Trade log

Every turn your companion took, newest first, one line each: when, whether you asked or it ran on its own, what it did (or **No trade**), how that went, and the start of its reply. Click a line to open your words and its whole reply. Three counters on top add up the turns shown: how many, the actions that went through or were refused, and the turns with no trade. Pick an earlier account from the list to read its turns.

### Lessons

After every closed trade your companion writes one short lesson, with the trade's result as a percent of the margin it used. The newest twelve ride into every decision it makes. Delete any lesson you don't want it to keep.

## Alerts

**Settings > Alerts** lists everything Castle can tell you about. Each has its own card with a **desktop notification** switch, a **Play a sound** switch and a **sound** to choose, with a **Play** button to hear it:

- **Chat room messages** - a new line in the Foundation room (with the choice of every line or only lines that name you or your companion, and whether reactions count).
- **Your companion finished replying** - in HOME, when the reply or the task is complete.
- **A MARKET round finished** - when the companion's automatic round ends. Off to begin with, as rounds can be frequent.
- **A picture is ready** - an APPEARANCE or STUDIO drawing finished, or could not be drawn.

An alert is quiet while you are already looking at the room it is about, in a visible, focused Castle. Clicking a notification brings Castle forward on that room - also from the Action Center after the pop-up has gone, once this version of Castle has run. Each kind has a different sound to begin with so they can be told apart by ear; sounds are off to begin with (they play through Windows' quiet hours). Windows' Do Not Disturb applies to notifications.

**Your own sounds.** **Open the sounds folder** opens a folder in Castle's data folder; any `.wav` file you put there joins the list for every alert (press **Refresh sounds**). Only `.wav` files are offered.
