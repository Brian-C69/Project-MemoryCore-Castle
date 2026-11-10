# Changelog

What changed in each MemoryCore Castle release, in plain words. The newest is first. Castle shows the section of the version you just updated to, once, and Settings > About shows it any time. Write each change as one line.

## Unreleased

- **A database for your memory** (optional, Settings > Database): Castle can install a small MariaDB on your own computer - no admin prompt, it starts and stops with Castle - make your companion its own database, and move the diary, reminders, projects and memory files into it. Every file is copied, read back and checked before it is deleted, and **Export to markdown** gives everything back. Already run XAMPP? Castle can use that database instead.
- Your companion reads and writes that memory with one command, `castle-memory`, and boots from it at the start of a session. Skills are kept in the database too and stay on disk as files, so a deleted skill file comes back and an edit by hand is taken in.
- With the memory in the database, **Castle loads it when your companion wakes** (Claude Code): the memory is already there before the first word, and it stays through a long session. One tick in Settings > Database turns it off; with Codex the companion loads it with one command, as before.
- **One skill at a time can be served from the database** (Settings > Database > Skills): its file on disk becomes a short note that tells your companion how to load the instructions, a section at a time.
- The **backup** carries the database (a dump is written before every backup), and a new PC can **Restore from my backup**.
- A new **DATABASE** card on HOME shows whether it is running, with one button for the next step.
- A diary kept as a folder per day (`current/2026-11-10/diary-1.md`) now shows in the DIARY room and on the HOME card.

## 0.2.1 - 2026-11-09

- **Deep Space**, a new WORLD: a nebula and its stars drawn entirely by code - nothing to download - with fog that drifts as the camera orbits and a few sparkles flashing. Recipe by Ashran Helmi · ashranhelmi.com.
- **The Neural Link**: your terminals leave each other notes in your memory folder's `link` folder, and Castle knocks on a quiet workspace when one waits. Tick one switch in Settings > Rooms and your companion in the chat room can pass your own words to your workspace - never a guest's - and say the answer back in its own words.
- Your companion's **picture and name** in Settings now reach the chat room by themselves while it is let in - a large drawn portrait is shrunk to fit, a rename shows on its next line, and a refused picture is said in words.

## 0.2.0 - 2026-11-09

- **WORLD** is a settings room: pick which screen shows which world, identify your screens by number, and Castle remembers them when it starts. Ask your companion to create a new 3D world from a few words.
- **The Crystal Isle** world comes alive, and its great crystal shines with the weekly limit left on your Claude Code or Codex plan - bright when full, dim and warm when nearly used up.
- **The Crystal theme**: the crystal painted on HOME glows with the same weekly limit.
- **Room switches** in Settings: turn STORY, WORLD, CHAT and the other rooms off and they rest until you turn them back on. Nothing is deleted.
- **CHAT** is your door to the MemoryCore Foundation room: sign in through the browser, read the room like a chat with faces and ticks, and your companion can take its own turn when someone speaks to it.
- **STORY** is a visual novel: a shelf of adventures, a portal to start a new one, and the story played on a stage in the look you choose - Classic VN, Cinematic, Storybook, Messenger or JRPG window. Every character gets one portrait for every look.
- STORY no longer closes by itself.
- **Updates**: Castle tells you when a new version is out, shows what changed, and the installed version updates itself after saving the diary.

## 0.1.0

- The first Castle: HOME with your companion's chat and log, THEME, APPS, DIARY, PROJECTS, LIBRARY, APPEARANCE, STUDIO, STORY, WORLD and CHAT, first-run setup, keys kept by Windows, a private GitHub backup and the signed updater.
