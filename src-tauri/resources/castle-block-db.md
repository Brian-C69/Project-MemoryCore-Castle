### The memory database

This memory lives in a database on {{USER_NAME}}'s own computer, not in markdown files. You reach it with one command, **`castle-memory`** (in PowerShell: `& "$env:CASTLE_TOOLS\castle-memory.cmd" <verb> ...`). It prints one plain result line, or the text you asked for.

- **The diary, reminders and projects are rows now.** Never create or edit `daily-diary/*.md`, `main/reminders.md` or `projects/*.md` - those files are gone, and a new one would not be read. When a skill or an older instruction tells you to write one of them, do the same thing through `castle-memory` instead.
- `save diary`: write the entry to a temporary file, then `castle-memory diary add --date <YYYY-MM-DD> --title "<a title for the session>" --file <that file>`, then delete the temporary file. Read back with `castle-memory diary list` and `castle-memory diary show --date <YYYY-MM-DD>`.
- Reminders: `castle-memory reminder add --title "..." [--due <YYYY-MM-DD>] [--detail "..."]`, `reminder list`, `reminder done --id <n>`.
- `save project`: `castle-memory project add --name "..." [--status "..."] [--summary "..."] [--file <notes file>]` (`project update` changes one that exists), `project list`, `project show --name "..."`.
- Long text always goes through `--file`, never on the command line.
- If `castle-memory` says the database could not be reached, tell {{USER_NAME}} kindly that the database is stopped and that **Start** in Castle's Settings > Database (or the DATABASE card on HOME) brings it back. Do not fall back to writing files.
- {{USER_NAME}} can get everything back as markdown at any time with **Export to markdown** in Settings (`castle-memory export --to <folder>`).
