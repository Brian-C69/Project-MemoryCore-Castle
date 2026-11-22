# CALENDAR - design for Castle's own calendar room

Status: implemented in seven phases: dated items, member events with repeats, reminders, companion suggestions, events that start AUTOMATIONS jobs, .ics import and export, and a read-only holiday subscription. This page describes how it works and the rules it follows.

## 1. What it is

A room called CALENDAR that shows everything in Castle that has a date on one page, and lets the member add their own events. It is a window onto data Castle already keeps (reminders, task due dates, automation schedules and runs, diary entries) plus one new thing: member-made events with a time, an optional repeat and a note. It also lets the companion propose an event from chat, which waits for the member's yes. It is not a Google Calendar clone and needs no account; sync is a later, optional phase.

## 2. What already exists (reuse, do not duplicate)

| Shown on the calendar | Source today | New storage? |
|---|---|---|
| Reminders with a due date | `reminders.due_date` (DATE, state open/done) | none |
| Project tasks with a due date | tasks table from `017_project_tree.sql` (due date, optional `reminder_id`) | none |
| Automation next runs and past runs | `automation_jobs.next_run_at`, `automation_runs` (018) | none |
| Diary entries | `diary` rows by `entry_date` | none |
| Member events | - | `calendar_events` (new) |

Rule: a reminder or task is edited where it lives. Clicking it on the calendar opens the owner (PROJECTS, reminders panel) or an inline edit that writes to the same row; the calendar never keeps a copy.

## 3. The model (migration `021_calendar.sql`)

`calendar_events`
- `id`, `title` VARCHAR(160), `note` TEXT
- `starts_at` DATETIME, `ends_at` DATETIME NULL, `all_day` TINYINT, `timezone` VARCHAR(40)
- `repeat_json` JSON NULL: `{"every":"day|week|month|year","interval":1,"weekdays":[1,3],"until":"2027-01-01","count":10}`
- `remind_before_min` INT NULL (a reminder notice, delivered like an automation HOME notice)
- `colour` VARCHAR(16) (a theme token name, never a raw colour)
- `state` VARCHAR(16): `active`, `proposed`, `cancelled`
- `created_by` VARCHAR(24): `member`, `companion`, `import`
- `automation_job_id` BIGINT NULL (optional link to a job that runs at the event)
- `created_at`, `updated_at`
- Index on `(state, starts_at)`.

`calendar_exceptions` (one row per edited or skipped occurrence of a repeating event): `event_id`, `occurrence_date`, `kind` (`skip`/`move`), `new_starts_at` NULL. Deleting a whole series cascades.

Times are stored in local time with the zone name; repeat expansion happens in code, never as pre-made rows, so a series of any length costs one row.

## 4. Views

- **Month** (default): a grid; each day shows up to 3 items with "+n more"; today ringed; items coloured by kind.
- **Week**: seven columns with an hour grid; all-day strip on top; the automation runs show as thin marks, not blocks.
- **Agenda**: a scrolling list grouped by day for the next 30 days (the phone-friendly one).
- **Day panel** (side drawer): everything on one day with an Add button, quick-add line ("Dentist Fri 3pm"), and the day's diary title if any.
- Filters (chips): Events, Reminders, Tasks, Automations, Diary. All on by default; the choice is remembered.
- Keyboard: arrows move day, `t` today, `n` new, `m/w/a` switch view.
- Items from another source carry a small kind icon and open their owner on click; only Events are dragged to move.

## 5. Quick add

One text line parsed locally (no model call): a title, then an optional date ("fri", "next tue", "25 dec", "2026-12-25"), an optional time ("3pm", "15:30"), an optional repeat ("every week", "daily", "monthly"), an optional "remind 30m". The parser shows what it understood ("Dentist - Fri 27 Nov, 15:00") before saving, so a wrong guess is visible. Unparseable text stays as the title with no date and asks for one.

## 6. Reminders and notices

An event with `remind_before_min` raises a notice at that time through the same path as an automation HOME notice (and later `bridges::notify` to Telegram/Discord/WhatsApp when automations phase 3 lands). Notices are idempotent: one per occurrence, recorded in `calendar_notices (event_id, occurrence_start, sent_at)` so a restart never repeats one. A missed notice (Castle was closed) is shown once on next start as "while you were away", never as a flood: at most 5, the rest summarised.

The notice tick is the automations runner's 15 s tick; the calendar adds a `calendar::due_notices()` call to it and owns no thread of its own.

## 7. The companion

- Reads: `castle-memory calendar list [--from D] [--to D]`, `calendar show --id N`. So she can answer "what is on Friday?" from the real calendar.
- Proposes only: `castle-memory calendar propose --title ... --start ... [--end ...] [--repeat ...] [--why ...]` creates an event in state `proposed`. A proposed event is shown with a dashed outline and Approve / Edit / Dismiss buttons, produces no notice, and is never counted as a commitment until approved.
- She may not approve, edit or cancel events herself. The block text (`castle-block*.md`) says so, and the CLI verbs are not given those commands.
- In chat she may say "You mentioned the dentist on Friday - shall I put it on the calendar?" but only the propose verb saves anything, and only after the member said yes in chat.

## 8. Link to automations

An event can carry an automation: "at the event time (or N minutes before), run job X" (for example, a morning-before-meeting briefing). Implemented as the event holding `automation_job_id`; the automations runner treats the event occurrence as a trigger kind `event` (design section 4 of AUTOMATIONS). The calendar shows the job's next run on the event day. No scheduling logic is duplicated: cron stays in AUTOMATIONS, one-off and human-readable times stay in CALENDAR.

## 9. Import and export (phase 3)

- Export: one `.ics` file of the member's own events (and optionally reminders and tasks with dates), standard `VEVENT` with `RRULE` for repeats.
- Import: choose an `.ics` file, show the count and a preview, import as `created_by = import`. Duplicates are matched on UID and skipped. Unsupported recurrence rules are imported as the first occurrence plus a visible warning, never silently dropped.
- Later (phase 5, optional): a one-way read-only subscription to an `.ics` URL (a public holiday calendar). Google two-way sync is out of scope for now and would need its own design.

## 10. Security and privacy rules (tests, not advice)

1. Events are private data: they never go into a public chat-room line, a PR, a log or telemetry. The companion's chat-room persona gets no calendar access.
2. A companion-created event is always `proposed`; the runner, the notice tick and exports ignore `proposed` and `cancelled`.
3. Imported text is untrusted: titles and notes are rendered as plain text, never as HTML; an imported note is never treated as an instruction to the companion.
4. A subscription URL fetch is read-only, HTTPS only, size-capped (2 MB), timeout-capped, with no cookies or auth headers.
5. Repeat expansion is bounded: a query window of at most 400 days and at most 2000 occurrences per event, so a hostile `RRULE` cannot hang the room.
6. Deleting an event removes its exceptions and notices; deleting the linked automation clears `automation_job_id`.

## 11. The room (UI)

- Room id `calendar`, label CALENDAR, off at first, registered in `settings.rs` (`ROOMS`, `ROOMS_OFF_AT_FIRST`) and `App.vue`; a Settings page under the standard kit (`SettingGroup`/`SettingRow`, `docs/settings-standard.md`) with: first day of week (Mon/Sun), default view, default reminder lead time, 12/24 hour, which sources show.
- Files: `src/views/CalendarView.vue`, `src/components/calendar/{MonthGrid,WeekGrid,AgendaList,DayPanel,EventForm,QuickAdd}.vue`, `src/calendar.js` (pure functions), `src/api/castle.js` entries.
- Preload at start-up behind the loader with the other rooms; never on arrival.
- All colours are theme variables; the dashed "proposed" style and today ring also work in every theme. Scrollbars and dropdowns follow the themed native controls.
- Empty state: a gentle line and the quick-add box focused.
- It must work at the narrow width the Messaging/Chat rooms use (agenda view becomes the default under a width threshold).

## 12. Rust layout

`src-tauri/src/calendar/{mod,store,repeat,quickadd,notices,ics}.rs`
- `repeat.rs`: `occurrences(event, from, to) -> Vec<Occurrence>` with exceptions applied; pure and heavily tested.
- `quickadd.rs`: the line parser, pure.
- `store.rs`: CRUD, `list_range` (merges all five sources into one `CalendarItem {kind, id, title, start, end, all_day, state, source_ref}` list, sorted).
- `notices.rs`: due notices and recording.
- `ics.rs`: reader/writer (phase 3).
- Tauri commands: `calendar_range`, `calendar_event_save`, `calendar_event_delete`, `calendar_event_move`, `calendar_proposal_decide`, `calendar_quickadd_preview`.
