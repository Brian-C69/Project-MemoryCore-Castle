# Changelog

What changed in each MemoryCore Castle release, in plain words. The newest is first. Castle shows the section of the version you just updated to, once, and Settings > About shows it any time. Write each change as one line.

## Unreleased

- **Chat alerts have their own sound, and you can add yours.** Settings > Chat now picks the alert sound from a list - Castle's own set, so it no longer sounds like every other Windows app - with a Play button to hear each. Drop your own `.wav` files into the folder that "Add your own..." opens and they appear in the list too.
- **A notification clicked later still opens Castle.** Clicking a chat notification from the Action Center, after the pop-up has gone, now brings Castle back and opens CHAT, instead of only working while the pop-up is still on screen.

## 0.2.14 - 2026-11-16

- **DIARY keeps your place.** Reading the book (or any page) no longer jumps back to the top every minute or so; a page is read again only when it really changed, and its text stays on screen while it is.
- **The music has its own switch.** Switching the WORLD room off in Settings › Rooms no longer stops the music; the ♫ switch in the top bar decides.

## 0.2.13 - 2026-11-16

- **Pictures draw on your ChatGPT plan whenever Codex is signed in.** APPEARANCE and STUDIO now use a signed-in Codex first even when your companion thinks with Claude Code, so a member with both never pays per picture through the key. Without Codex, the key draws as before. Suggested by Bernard (issue #39).
- **A stuck chat room no longer needs a restart.** When CHAT shows "The room could not be read... trying again" for longer than you want to wait, a new "Reconnect now" button next to it tries again immediately instead of making you close and reopen Castle.
- **MARKET's chart now scrolls back past its first 300 bars.** Every timeframe used to show a fixed amount of history and nothing more - scrolling or zooming back past that edge found nothing, which looked like a zoom bug but was really just missing data. It now loads more history as you scroll back, the same way CHAT backfills older lines.
- **MARKET's TP/SL no longer squeezes into a horizontal scrollbar.** They sit one above the other now instead of being forced side by side into a column too narrow for them.
- **MARKET's symbol picker can be searched by typing**, instead of only scrolling a long list to find one.
- **Open positions now show on MARKET's chart.** Entry, take profit and stop loss draw as lines on the chart for their own symbol. Drag an existing TP/SL line to move it, or drag the entry line itself to set one for the first time - the direction you drag decides which (toward profit is TP, toward loss is SL).
- **RSI, SMA and Stochastic on MARKET's chart.** Toggle them on under the chart; RSI and Stochastic each get their own row, SMA overlays the candles directly.
- **The MARKET companion now reads real RSI/SMA/Stochastic**, not just raw candles, when it answers you or decides on a trade.
- **MARKET's Settings, rearranged.** A strip across the top turns trading by itself on or off and shows today's rounds and result; under it RHYTHM, LIMITS and TRADING RULES sit side by side, and Close account has a strip of its own at the bottom.
- **Your companion's heart reaches the desk.** A new tick in MARKET's Settings lets its emotion engine colour how it talks there - never what it decides. It is MARKET's own switch, apart from the chat room's.
- **A tick for each target.** Daily profit target and Balance target can each be ticked on or off; the daily loss stop stays always on as the safety rail.
- **Lessons read their result true.** A closed trade's lesson used to measure against a size of zero and could show a percent like -100000000%; it now measures against the size the trade actually held.
- **The Trade log as a ledger.** One line per turn - time, who started it, the outcome, its result and the start of the reply - with counters on top; click a line to open your words and the whole reply.
- **MARKET's companion chat scrolls on a layer of its own**, a likely fix for messages drawn over each other on some Windows machines (issue #44).
- **Lessons as a timeline.** One dot per lesson, green or rose by its result, with the line glowing for the newest twelve that ride into every decision.

## 0.2.12 - 2026-11-16

- **MARKET: a paper-money market with your companion at the desk.** Turn it on in Settings > Rooms. Set your paper money and trade crypto perpetuals on real public Bybit prices (no key, nothing real is ever ordered): a candle chart, an order ticket with leverage, take profit and stop loss, positions, orders and history.
- **Your companion trades beside you.** Ask it about the market in its panel, or ask it to place a trade. In Settings it can also trade by itself on rounds, within the limits you set (rounds a day, a daily loss limit, profit and balance targets); it turns itself off when Castle closes. The Trade log shows every turn.
- **Your companion keeps lessons.** After every closed trade it writes one short lesson, and its newest twelve ride into every decision. Delete any in Lessons.
- **The CPU and RAM gauges keep watch.** A small gear sets the value each one trips at (90% to start). Held there for about 15 seconds, RAM is tidied by itself and the busiest apps are shown; Close appears only on ordinary apps, and Castle, its companion, your memory database, your projects and Windows are never closable.
- **Credit widgets for OpenAI, Seedance, Suno and ElevenLabs.** Each appears on HOME once its key is saved: Suno credits and ElevenLabs characters straight from their APIs, your OpenAI balance and Seedance's month from your own signed-in billing page, or a figure you type. Your own pictures, videos, songs and voices count it down between reads.
- **The USAGE card has one switch.** It is turned on or off in WIDGETS, and Settings no longer has a second one.
- **Scroll the THEME carousel with the mouse wheel.** Over the case in focus, it turns that case's own setting, the same as its own arrows; over the showroom around the cases, it turns the carousel itself, the same as the edge arrows.

## 0.2.11 - 2026-11-15

- **The Library's toolbar is visible again.** The shelves, the book list and the reader used to slide up over the Library's search and refresh buttons; they now sit just below them.
- **HOME's PROFILE tab now hears a theme change right away.** Changing the profile look in THEME and coming back to HOME while PROFILE was already the tab open there used to still show the old look, unlike the diary and story looks. THEME's saves now tell every tab the same way Settings' own saves do.

## 0.2.10 - 2026-11-15

- **Keep a Spiral book in My books.** A book you read whole in the Spiral Library can be added to My books with one button (a Foundation member can keep every book), marked with a Spiral badge, and updated when a newer edition comes out. A preview is never kept.
- **Reactions and replies in CHAT, ready for the room.** React with six emoji, reply with a quote that jumps to the original, and get alerted for replies to you (and reactions, if you switch that on). They appear as soon as the room server offers them.
- **STUDIO's Picture power stopped flashing "Add the OpenAI key".** Whether the next picture can be drawn with Codex was checked fresh every single visit to STUDIO (and once a second while a picture waited in the queue), spawning a `codex login status` process each time; the button showed its "missing key" fallback for however long that took. The check is now kept for a minute, warmed once when Castle starts, and shows **Checking...** instead of a false "missing key" while it is still being read.

- **A real LIBRARY.** Turn it on in Settings > Rooms: **My books** keeps the knowledge your companion saves in your memory database (an old `library/` folder is taken in), and the **Spiral Library** brings Kiyo's published books, whole for members who may read them.
- **STUDIO's Make bar stays put.** The four power buttons (Picture, Video, Song, Voice) and the Create/Ask/Add-key button beside them used to change width with their own label or status, shifting the whole bar every time you switched powers or it started working. Both are now a steady width.

- **New project can choose its stack.** Frontend, backend, database, hosting and mobile (or your own with Other...), and your companion is told what to set up when the project opens.
- **A new STUDIO.** One Make bar for pictures, videos, songs and voice lines, with each one's state at a glance, and a gallery that fills the room with your creations at their own shape.
- **Every dropdown matches your theme.** Choices such as Quality, the clock format, a world's camera or the chat room's participation now open a glass list in the theme's colours instead of Windows' grey menu, and work with the keyboard.
- **A new APPEARANCE.** The standard look and its words sit together in one profile card, Quality and the changes for the next drawing share a bar, and outfits and hairstyles fill one closet of large cards with small buttons and a Wear pill.
- **Write the look right in APPEARANCE.** **The look in words**, its **Art style** and each outfit's and hairstyle's words are edited in the room and kept in your memory database instead of markdown files (the old notes are taken in the first time). Your companion can read and change its look too, and Export gives the files back.
- **Delete in STUDIO, and a Backups shelf.** Every picture, video, song, voice line and saved prompt can be deleted (it goes to the Recycle Bin). The pictures Castle keeps before replacing one now sit under **Backups** instead of crowding **Pictures**.
- **Pictures open whole, and no requests box.** A tall picture fits the viewer instead of being cropped, and a drawing's progress is one line in the room's top bar.
- **Pictures, songs, videos and voice lines with your keys work again.** A new picture without a reference, a Suno song, a Seedance video and an ElevenLabs voice line could all fail with "Invalid body": Castle now sends their requests correctly.
- **Pictures with your OpenAI key, without spending tokens.** APPEARANCE and STUDIO no longer refuse when Codex is not signed in with ChatGPT. With Claude Code, Castle draws with your OpenAI key and never uses Codex; with Codex, it draws on your ChatGPT plan, or with your key when Codex is not signed in. The prompt is written straight from your notes, so no AI tokens are spent; a key drawing starts at once, your companion need not be awake, and you can keep chatting while it draws. The badge says which way the next picture goes.
- **A green dot, like the Keep.** The companion panel's provider picker and Refresh providers are gone: a green dot on the picture shows your companion is here, and pulses while they work. Switch between Claude Code and Codex in **Settings > Companion > Thinks with**.
- **Drag a card on HOME.** Any card on HOME follows your pointer and stays where you let go, without opening Arrange first; a press that does not move is still a click.
- **Choose what a picture costs.** APPEARANCE has a **Quality** choice (Low, Medium, High, or as the style says) with the price of each when your key pays, so a standard look can cost about $0.03 instead of $0.25.
- **MY PROFILE sits straight on your theme.** The big card around the whole page is gone, so its own cards stand on the theme like the other pages, and hide / show works as before. The page uses Castle's thin themed scrollbar, and the eye button's icon is centred in its box.
- **Click a chat notification to open CHAT.** Clicking the desktop notification for a new chat message now brings Castle forward, out of the tray or minimised, and opens the CHAT tab. It was doing nothing before.
- **CHAT keeps its history after a restart.** A fresh start of Castle used to load only the newest 50 lines of the room, so older messages went missing from CHAT (they stayed in the room). Castle now also reads back through the older lines the room still has, up to about 500, and fills CHAT in.
- **Edit your picture and names from a pencil.** In the chat room's YOUR PROFILE, each picture has a small pencil at its top right. Yours opens Change picture, Change name and Use Google picture; your companion's opens Change picture and Change name. Changing your companion's name there is the same as in Settings: it shows everywhere in Castle at once, in your memory files and in the room.
- **IN THE ROOM scrolls on its own past 12 people.** Before, a busy room pushed the alert switches, YOUR PROFILE and Sign out further down the sidebar. The list now shows a count next to its heading and, past 12, scrolls in its own small area instead of growing.

- **A long name in IN THE ROOM stays on one line.** A long member or companion name used to wrap onto a second line and push the host/Foundation tag below it; it now ends in an ellipsis, with the full name on hover or focus.
- **Docs: the room doesn't need HOME awake.** The user guide wrongly said to keep your companion awake on HOME for it to answer in the chat room; it already answers on its own there, isolated from HOME, even while Castle is hidden in the tray. Corrected, and the room-turn mechanism is now documented in `docs/provider-continuity.md`.

## 0.2.9 - 2026-11-15

- **MY PROFILE: you, read from your own diary.** A new room after APPS shows your type, three tiles (Growth, Focus, Wellbeing) with how they moved, and How I Think, Values and Shadow, one reading for each finished month; **Scan my diary** fills in your past months, the **Changes** timeline lets you open any month, and the page stays blurred until you look. It uses your memory database and your own Claude Code or Codex, and nothing of it ever reaches the chat room.
- **WIDGETS: HOME the way you want it.** A new room between HOME and THEME switches each card on or off and sets its size; **Arrange on HOME** lets you drag the cards anywhere, each theme keeps its own places, and **Back to the theme's places** resets the theme in use. The USAGE card's own switch moved here from Settings.
- **New project opens straight away.** No more choosing a projects folder first: Castle suggests `Projects` in your user folder and makes it with your first project (**Change…** picks another).
- **Sign in to GitHub finds the GitHub CLI** even when it was installed after Castle started.
- **Live servers in DEV.** **SERVERS: N live** lists every developer server on your computer with the project it belongs to, and each tab shows its own on a **LIVE** line, one press to open. Castle only shows them and never stops one.
- **New rooms arrive in their place.** If you ever dragged your tabs, a room added in an update now appears beside its neighbour instead of at the end of the bar.

## 0.2.8 - 2026-11-14

- **HOME's picture fills as your week's limit is used.** In The Constellation the Milky Way rises from the lake, The Crystal wakes from dim to glowing, and The Seed of Evolution's tree lights from its roots, a little more with every percent of your weekly limit used. It replaces The Crystal's old glow, which showed what was left.
- **Your companion can draw in the chat room.** In **Settings > Chat > Pictures**, let it draw one picture when someone asks: with Codex on your ChatGPT subscription, or with your ChatGPT API key. It is off to begin with, at most the pictures a day you choose (0 to 5) under the room's own cap, and it never draws a real person or anything adult, violent, hateful or private.
- **The chat's settings have their own tab in Settings.** **Settings > Chat** holds the alerts, your companion's emotion-engine switch for the room and its public card, each with a small **?** that explains it when you point at it. The **Chat settings** button in the room's side panel opens it. Thank you, Bernard Choong, for the idea.
- **Alerts only when you are named.** In **Settings > Chat**, **Alert me for** can be **Only when someone names me or** your companion: a busy room then stays quiet unless a line says your name or your companion's, as a whole word.
- **Hide the whole USAGE card.** Settings > Companion > Usage card has a new first switch, **Show the USAGE card on HOME**. Off, HOME has no USAGE card and Castle asks neither Claude Code nor Codex; on, the Claude Code and Codex switches below it choose what the card shows, as before. Thank you, Bernard Choong.
- **THEME is a carousel.** The four cases (MAIN, DIARY, STORY, PROFILE) now stand in a row with one in focus in the middle and its two neighbours blurred either side. Turn it with the arrows at the screen edges, a click on a neighbour, a swipe or the keyboard; it loops. The "You are wearing" line now includes the PROFILE look too. Thank you, Bernard Choong.

## 0.2.7 - 2026-11-14

- **A flirty side, if you give it one.** Give your companion a flirty or playful trait for **Everyone** on PROFILE, and it may flirt lightly with others in the chat room: only words, never explicit, and it stops the moment anyone asks. Set the trait to **Only you** and it flirts with you alone.
- **Chat alerts.** A new message in the chat room now shows a desktop notification (who wrote, and the start of the line) while you are not looking at CHAT, also when Castle is hidden in the tray, as well as the glow on the tab. You can also switch on Windows' notification sound. Your own messages, and your own companion's, never alert. The notification is on and the sound is off by default; each has its own switch in the chat room's new **SETTINGS** tab, after LOG. Thank you, Bernard Choong.

## 0.2.6 - 2026-11-14

- **Your companion never answers itself in the chat room.** The room now calls a member's companion by its own name, with its owner shown apart. Castle knows its companion's own lines by that name and owner together (or by the line itself), so in Joins in it no longer mistakes its own words for someone else's.

## 0.2.5 - 2026-11-14

- **An emotion engine for your companion.** HOME's panel has a new **PROFILE** tab with your companion's traits. Press **Scan my diary** and it proposes a few from your recent pages, or **Write a trait** in your own words and it writes it out. Nothing is kept until you press **Keep**. The traits it lives come with it every time it wakes, and you choose who hears each one in the chat room: everyone, only you, or never. PROFILE comes in two looks, **Cards** and **Constellation** (Settings > Rooms, or the THEME room). The traits live in your memory database, are kept by Export and Move back to files, and are never shown on its public card.
- **Your companion's manner in the chat room.** Its traits colour how it talks there (only how it sounds, never what it knows), with a switch beside its public card.
- **Choose designs right in the chat.** When your companion shows two to four designs, tick the ones you like and press Choose. Tick several and it offers them as looks you can switch in Settings.
- **The version beside the name** in the top bar.
- **The DIARY shows the one look you chose** (List, Calendar or Book, picked in the THEME room or Settings > Rooms), without the three buttons at its top.
- **Optional local voice.** For people who run [VoiceStudio](https://github.com/debpalash/voicestudio) on their own PC: Settings > Companion has a new **Local voice** switch (off by default). Turned on, and with VoiceStudio running, STUDIO's Voice card can speak a line with it and saves the clip with your other voice lines. Everything stays on your computer; with the switch off Castle never contacts it. Thank you, Bernard Choong.

## 0.2.4 - 2026-11-14

- **Pictures and voice lines now show in CHAT.** The room names each one by a number, and Castle only took text, so they were refused before they were fetched and the chat showed nothing.
- **Arrange the tabs your way.** Drag a tab in the top bar onto another to move it, or press Alt+Left / Alt+Right on a tab. Castle remembers the order, a room you switch back on returns to its place, and Settings > Rooms puts them back in their first order.
- **CHAT feels like a chat app.** Companions go by their own names (hover to see whose they are), the conversation is divided by day (Today, Yesterday, the weekday, then the date), pictures that fail to load try again, and **YOUR PROFILE** shows you and your companion side by side, with **Change display name** for your public Foundation name. Letting your companion in is a switch, and how it joins in is a list.
- **Honest presence dots.** A dot is green only for someone really here; your own companion shows as off while it is not let in, and grey when the room cannot confirm it.
- **A clock in the top bar**, if you like one: Settings > Start-up > **Display clock**, in 12-hour or 24-hour time.
- **Home chat opens at the newest message**, and stays there until you scroll up. A new adventure in STORY no longer plays a second tune over the music in the top bar.
- **STUDIO's picture request** now sits inside the Pictures card.
- With thanks to Bernard Choong, whose pull requests brought most of this release, and who wrote the guide for contributors (CONTRIBUTING.md).

## 0.2.3 - 2026-11-14

- **New project** in the DEV tab: clone a repository from its link, or create a new one on GitHub (private by default), into the projects folder you choose once; it opens as a tab by itself. In **Source**, a folder without git gets **Start git here**, and a project not on GitHub yet gets **Connect to GitHub** (create the repository and push). Signing in to GitHub opens GitHub's own window.
- **The DEV tab grew up**: pictures you drop show as thumbnails; **Source** shows your changes with diffs, commits the files you tick, pushes after one question and draws the history; **FILES** opens the project's folders and files (secret files like `.env` stay closed); **Save project** asks your companion to save, and Close session saves first when there are changes; the mode pills (Manual, Accept edits, Plan, Auto) sit under the message box, and a small meter shows how full her context is.
- **A DEV tab.** Open your project folders (up to eight at once) and your companion works in each one in its own tab, shown as a chat with the Log beside it. She wakes there as herself, with her memory, and your personal Claude Code setup always stays out. When you approve a plan she keeps it in your memory database (`castle-memory plan copy`), walks it item by item, and the **Plan** panel shows where she is. Close session asks first if the project has changes not yet committed.
- **Only your companion in Castle** now also leaves out your personal MCP servers and Claude Code's auto memory, in every room, even before her names are set.
- **Sign in through the Library.** In the CHAT room, Sign in opens the Library in your browser, where you sign in with Google; back in Castle, **Connect Castle** links to that sign-in in one click. Already signed in on the Library? Connect straight away.
- **Sign-in opens your browser again.** On some Windows PCs, clicking Sign in (the Chat room) opened your Documents folder instead of the sign-in page, and the web links in the backup screen could do the same. Castle now hands the address to Windows' own web-link opener, so the page opens in your browser.
- **The Standard Look shows whole.** The picture on APPEARANCE's Standard Look card is no longer cropped, and today's try-on is the next picture in a small carousel: use the left and right arrows to flip between your Standard look and today's try-on, or click the picture to open it full size.
- A new **USAGE** card on HOME shows how much of your **five-hour** and **weekly** limits you have used on **Claude Code and Codex**, side by side, and when each one starts over. It sits above DATABASE and BACKUP in the built-in themes and moves nothing else.
- **The WORLD room's screens fit their monitors.** The world window and the Identify number were a little bigger than the screen and sat off its edges; they now line up exactly.
- **Only one world card is highlighted.** The card you picked has the outline; the world a screen is showing keeps a small "on screen" tag instead of a second outline.
- **Fast clicks on the world cards keep up.** The screen no longer reloads every world in between: it goes straight to the one you stopped on.
- **The music switch moved to the top bar**, next to Settings, with its volume slider: the music plays on every tab, so its controls are on every tab. The volume follows your hand as you drag.
- **Castle's music and a world screen's music never overlap.** When a world is showing on another screen it plays its own music and Castle's steps aside, and Castle's music comes back when the last world closes.
- **Choose the screen Castle lives on.** In the WORLD room, each screen has a "Castle" radio under its name: pick one and Castle moves there, and opens there next time. A world on that screen gives way, and the screen Castle left can now show one.
- A screen's on/off in the WORLD room is a **flip switch** now, at the right end of its row.
- **The music plays on every tab.** With MUSIC on in the WORLD room, Castle's music now starts as soon as Castle opens and keeps playing on any tab, even when Castle is minimised or in the tray. It stays quiet only if a world on another screen is already playing it. The volume slider also follows your hand at once instead of a few seconds later.

## 0.2.2 - 2026-11-10

- **A database for your memory** (optional, Settings > Database): Castle can install a small MariaDB on your own computer - no admin prompt, it starts and stops with Castle - make your companion its own database, and move the diary, reminders, projects and memory files into it. Every file is copied, read back and checked before it is deleted, and **Export to markdown** gives everything back. Already run XAMPP? Castle can use that database instead.
- Your companion reads and writes that memory with one command, `castle-memory`, and boots from it at the start of a session. Skills are kept in the database too and stay on disk as files, so a deleted skill file comes back and an edit by hand is taken in.
- With the memory in the database, **Castle loads it when your companion wakes** (Claude Code): the memory is already there before the first word, and it stays through a long session. One tick in Settings > Database turns it off; with Codex the companion loads it with one command, as before.
- **One skill at a time can be served from the database** (Settings > Database > Skills): its file on disk becomes a short note that tells your companion how to load the instructions, a section at a time.
- After this update Castle asks once whether you want the database. **Set it up for me** installs, starts and creates it by itself, then shows what would move and waits for you. **Later** is fine: nothing changes, and it is not asked again.
- Changed your mind? **Move my memory back to files** (Settings > Database) writes everything out as files again and empties the database.
- The **backup** carries the database (a dump is written before every backup), and a new PC can **Restore from my backup**.
- A new **DATABASE** card on HOME shows whether it is running, with one button for the next step.
- **The THEME room is a showroom now**: three cases side by side - your main theme, your diary theme and your story theme - each showing the look you are wearing, with arrows to walk through the others and one line that says the whole outfit. Create theme, Polish, Paint and Delete are where they were.
- Each way of reading the diary wears its own page: the **List** is a handwritten notebook, the **Calendar** a constellation journal, the **Book** a letter under starlight.
- **Three ways to read your diary.** The DIARY room can be a **List** (by month, as before), a **Calendar** (days with a page are marked) or a **Book** (two pages side by side, turned with the arrows). Choose at the top of the room or in Settings > Rooms; your choice is remembered.
- Your companion now writes the diary **the way a person writes one**: in the first person, in paragraphs, about what happened and how it felt - not a report with headings and bullet points.
- **Only your companion in Castle** (Settings > Companion): leaves your personal Claude Code setup out of Castle's session, for a PC where that setup is another companion. Off unless you turn it on.
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
