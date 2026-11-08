# MemoryCore Castle - User Guide

MemoryCore Castle is a full-screen desktop for your MemoryCore companion. It opens your own MemoryCore folder, runs **Claude Code** or **Codex** inside it, and gives your companion a home on your screen.

> This guide grows with Castle. Screenshots are added with the first release.

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

---

## 1. Getting started

### What you need

- Windows 10 or 11
- **Claude Code** or **Codex**, installed and signed in with your own account. Castle has no account of its own; your companion uses the subscription you already have.
- A MemoryCore folder. If you do not have one yet, Castle can create it for you the first time it opens.

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

HOME is your desktop. Your companion's panel is on the left; the cards on the right read your memory and update by themselves every minute.

### The companion panel

At the top: your companion's picture (or the glowing crystal), their name, and the app they think with (**Claude Code** or **Codex**). Under it are the quick actions, then **CHAT** and **LOG** (see chapters 2 and 3).

### The cards

| Card | What it shows | Where it comes from |
|------|---------------|---------------------|
| **Clock** | The time and date | Your computer |
| **CPU / RAM** | How busy your computer is | Your computer |
| **TODAY** | Your open reminders, with their due dates | `main/reminders.md` (MemoryCore's Reminders) |
| **PROJECTS** | Your active projects, most recent first. Click one to open its file | `projects/project-list.md` (MemoryCore's Projects) |
| **DIARY** | Your newest diary entry. Click it to open the page | `daily-diary/` |
| **GROWTH** | How many memory files, days since your first diary, and skills | Your MemoryCore folder |
| **BACKUP** | When your memory was last backed up, with **Back up now** | Your private GitHub backup (chapter 5) |

A card with nothing to show yet says so in a friendly line - for example, a new memory's DIARY card reminds you to press **Save diary** after a talk.

### Rooms that appear by themselves

Some rooms only show in the top bar once your MemoryCore has that feature: **DIARY**, **PROJECTS** and **LIBRARY**. Install the feature (ask your companion), and the room appears within a minute.

---

## 7. Themes

Open **THEME** (right after HOME) to change how Castle looks.

- Every theme is a card with a preview. Press **Use** to switch; the theme in use says **In use**.
- **Create theme:** give it a name, describe the mood and colours in your own words, and upload a picture (or leave it out for a soft gradient). Press **Create** - Castle reads the colours from your picture and makes a working theme straight away, with text kept easy to read.
- On your own themes:
  - **Polish** asks your companion to fine-tune the colours and layout from your words.
  - **Paint** asks your companion to paint a wallpaper for the theme (it tells you the cost first; with Codex it draws by itself).
  - **Delete** removes the theme (choose another theme first if it is in use).
- After your companion has worked on a theme, press **Reload themes**. If a theme cannot be read, it is listed at the bottom with the reason, and Castle keeps showing the theme you had.

Your themes live in Castle's settings folder (`%APPDATA%\com.memorycore.castle\themes`), one folder each with a `theme.json` and a wallpaper, so you can also make or share themes by hand - see `MAKE-IT-YOURS.md`.

---

## 8. Apps, Diary, Projects and Library

### APPS

Your Start-menu apps, with the ones you use most at the top. Type to search, then press **Enter** or click to open an app.

### DIARY, PROJECTS and LIBRARY

These rooms let you read your companion's memory without leaving Castle (they appear once your MemoryCore has the feature):

- **DIARY** - every page, newest first, grouped by month. Older diaries are read too.
- **PROJECTS** - your active and archived projects.
- **LIBRARY** - your saved knowledge by section, wherever your library lives.

Pick a page on the left to read it on the right; type in **Search** to find one. **Show file** opens the page's folder. The rooms only read - your companion is the one who writes your memory. Clicking a project or the diary on HOME opens it here.

---

## 9. Studio

Your companion can make pictures, short videos, songs and voice lines. Just ask in Chat - for example *"Draw me a picture of our garden at night"* or *"Write and sing a song about today"*.

### The four cards

At the top of **STUDIO**, each power shows its state:

| State | Meaning |
|-------|---------|
| **Ready** | The key is kept, so your companion can make the real thing |
| **Built in (Codex)** | Codex draws pictures with your ChatGPT account - no key needed |
| **Prompt only - add a key** | No key yet: your companion saves the idea (the picture description, the lyrics, the words) instead, and Castle tells you |

Press **Add the ... key** on a card to add it (see chapter 4, Keys).

### Before anything costs money

Your companion always tells you the estimated cost first and asks before making a picture, video, song or voice line.

### When a key is missing

The idea is saved next to where the file would have gone (a `.prompt.md`, `.lyrics.md` or `.line.txt` file), and a note appears at the bottom right: *"Saved the lyrics instead - add the Suno key in Settings to hear it sung."* Add the key, then ask again to make the real thing.

### The gallery

- **Pictures** - click one to see it large; use the arrow keys to move between them.
- **Videos**, **Songs** and **Voice** - play them right in Castle. Songs are grouped by album.
- **Saved prompts** - the ideas saved while a key was missing, with **Show file**.

Everything lives in your MemoryCore folder (`media-generation/`, `music/`, and the story's own folders), so it is part of your memory and your backup.

---

## 10. Appearance

**APPEARANCE** gives your companion a face of their own, and clothes and hairstyles to change into.

### How drawing works here

Your companion draws the pictures. The top of the room says whether it can:

- **Ready to draw** - Claude Code with a ChatGPT (OpenAI) key.
- **Codex draws by itself** - no key needed.
- **No ChatGPT key - prompts are saved** - the picture's description is saved instead; press **Add the ChatGPT key** to draw for real.

Your companion tells you the cost first and asks before drawing. When a picture is saved, press **Refresh**.

### The standard look

Press **Draw the standard look**. Your companion draws a front portrait from `appearance/companion.md` - its look in words. If your MemoryCore already describes your companion's appearance (MemoryCore's Image Prompt feature), Castle starts from those words. Change the words any time (ask your companion, or edit the file), then **Redraw**.

The standard look also becomes your companion's picture in the panel, unless you chose your own picture in setup.

### Wardrobe and hair

- Press **+ New** on **WARDROBE** or **HAIR**, give it a name and describe it, then **Add and draw**. Your companion draws a turnaround sheet - front, side and back - using the standard look so the face stays the same.
- Click a sheet to see it large. **Redraw** asks for a new one.
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
