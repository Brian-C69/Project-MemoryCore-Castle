# APPS: the room launcher

APPS is one page with every room on it, so the top bar can stay short. The top bar holds HOME, APPS and the rooms the member pins (CHAT, PROJECTS and DEV to begin with).

## The page

- A flat grid of tiles, eight across on a wide window (six, four, then three as it narrows). Each tile is a room's icon with its name under it. Hovering or focusing a tile shows what the room is for in a small tip; the first two tips wait a moment, and from the third tile in a row they show at once, until the pointer has been off the tiles for a short while.
- A search box on the title row. Typing filters the rooms by name, Enter opens the first match, the arrow keys, Home and End move between tiles, Esc clears the search and then goes back to the room you came from. Opening APPS does not put the cursor in the search box; Ctrl+K (Cmd+K on a Mac) opens APPS from anywhere with the cursor there, and pressing it again inside APPS jumps back to the search. Ctrl+K is left alone in DEV, where it belongs to the terminal, and while a sheet is open.
- A pin on each tile (or P on the focused tile) puts the room in the top bar or takes it out. HOME and APPS cannot be unpinned. The pins are saved as `pinnedRooms` in settings; a settings file without it loads with the defaults.
- Badges on tiles say something is waiting: failed or timed-out AUTOMATIONS runs (or an amber dot when Pause all is on), chats waiting for approval on MESSAGING, something new in CHAT or STORY, and today's events and raised reminders on CALENDAR. They only read state Castle already holds.
- APPS shows every room. The per-room switches in Settings no longer hide a room from APPS or the top bar; the member pins what they use. A room whose MemoryCore feature is missing stays out.

## Naming

The room that lists the Start-menu apps was called APPS; it is now labelled **DESKTOP** (its id, `apps`, its most-used counts and its place in the tab order are unchanged). The launcher's own id is `launcher`. DESKTOP always lists the apps (it no longer depends on its room switch), does not put the cursor in its search box on opening, and has a Clear button for its search.

## Icons

One icon per room in `src/assets/rooms/<id>.svg`: a flat, hand-drawn SVG that draws in the theme's accent colour, so it follows every built-in and user theme. `src/roomIcons.js` maps a room id to its file; a room with no file shows a themed letter tile, so the launcher works with zero icon files, and a test checks that every room has one. The tile surface follows the theme too.

## Files

`src/views/LauncherView.vue`, `src/components/launcher/{RoomTile,LauncherSearch}.vue`, `src/launcher.js` (grouping order, filtering, keyboard movement, badges, tip timing; pure and tested), `src/roomIcons.js`, `src/assets/rooms/*.svg`; edits in `src/App.vue`, `src/components/TopBar.vue`, `src/views/SettingsView.vue`, `src-tauri/src/settings.rs`.
