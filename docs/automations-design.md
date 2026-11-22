# AUTOMATIONS - design for Castle's own job runner

Status: implemented in phases (schedule and companion, commands, Castle actions and phone delivery, message, file, web request and event triggers, follow-up chains, HTTP, companion proposals, and 24/7 operation while Castle runs). This page describes how it works and the rules it follows.

## 1. What it is, in one paragraph

A room called **AUTOMATIONS** where the member keeps a list of jobs. A job is a **trigger** (a time, a message, a file, a webhook, a Castle event), an **action** (ask the companion, run an allowed program, do a Castle thing, call a URL) and a **delivery** (tell the member on HOME, Telegram, Discord or WhatsApp, or save the result to a project). Castle runs the jobs, keeps a **run history**, and never does anything the member did not write down and switch on. It is a cron panel and a small n8n for one person's computer, built from parts Castle already has.

Non-goals for v1: a visual node editor, running jobs for other people, cloud hosting, running anything with admin rights, running text that arrived in a message.

## 2. Why Castle can do this cheaply (parts that already exist)

| Need | Already in Castle |
|---|---|
| Ask the companion something once, no tools | `oneshot::run` / `run_as` (own rules file, no tools, no MCP) - used by the room and the emotion engine |
| Ask the real companion with its memory and tools | `bridges::send_and_await` (types into the main session, follows its record) |
| Send to a phone | Telegram `send_message`, Discord and WhatsApp senders in `bridges/` (need one shared `notify`, see 6.3) |
| Notify on HOME | `alerts.rs` + the Alerts page |
| A schedule that works while Castle is closed | Sentinel's `--sentinel-tick` run by the system scheduler (`schtasks`, `sentinel/watch.rs`, `sentinel/hand.rs`) |
| Rows that the companions and the UI share | The Castle database (`memdb.rs`, migrations in `src-tauri/resources/schema/`, last is `017_project_tree.sql`) |
| Secrets | Windows Credential Manager (`key.rs`, `KEYS`) |
| A room, its off-at-first switch, its Settings | `settings.rs` `ROOMS` / `ROOMS_OFF_AT_FIRST`, `App.vue` `ROOMS` + `BUILT`, Settings standard (`docs/settings-standard.md`) |
| Pending-approval pattern for things the companion proposes | emotion traits (`trait propose`, member says yes on the PROFILE tab) |

## 3. The model

### 3.1 Job

```
automation_jobs
  id            INT AUTO_INCREMENT PK
  name          VARCHAR(80)  unique per member
  note          TEXT         what it is for, the member's words
  enabled       TINYINT      0 at creation; the member switches it on
  trigger_kind  ENUM('schedule','message','file','webhook','event','manual')
  trigger_json  JSON         see 4
  action_kind   ENUM('companion','command','castle','http')
  action_json   JSON         see 5
  delivery_json JSON         see 6 (list; may be empty)
  policy_json   JSON         timeout_s, max_output_kb, missed ('skip'|'run_once'), retries, only_when_castle_open, rate_per_hour
  next_run_at   DATETIME NULL   (schedule jobs; maintained by the runner)
  created_by    ENUM('member','companion_proposal')
  state         ENUM('active','proposed','archived')   a companion's proposal waits as 'proposed'
  created_at, updated_at
```

### 3.2 Run (the history)

```
automation_runs
  id, job_id FK, started_at, finished_at NULL,
  cause        ENUM('schedule','message','file','webhook','event','manual','chain')
  status       ENUM('running','ok','failed','timed_out','skipped','cancelled')
  summary      VARCHAR(400)   one line the panel shows
  output       MEDIUMTEXT     trimmed to policy.max_output_kb (default 64)
  error        TEXT NULL
  delivered    JSON           per delivery: ok/failed and why
```

Retention: keep the newest 200 runs per job; older rows are deleted by the runner once a day. A global **Pause all** switch (a setting) stops every trigger instantly and is shown on HOME's notice line when on.

### 3.3 Settings

`automations_on` (room on/off, `messaging`-style: the room is in `ROOMS_OFF_AT_FIRST`), `automations_paused` (kill switch), `automations_commands` (the allow-list, see 5.2). Nothing runs while the room is off.

## 4. Triggers

Every trigger yields a **payload** (a small JSON object: time, matched text, file path...). A payload is **data**. It is passed to the action as context or as an environment variable, **never** concatenated into a command line and never used to choose which command runs.

### 4.1 `schedule` (phase 1)

- Fields: `{ "cron": "0 8 * * 1-5" }` or `{ "every_minutes": 30 }` or `{ "at": "2026-12-01T09:00" }` (one-shot, then archived). The editor also offers presets ("Every morning at 8", "Every hour", "Every Sunday").
- Local time. DST: a time that does not exist is skipped, a time that happens twice runs once.
- Write a small 5-field cron parser (minutes, hours, day-of-month, month, day-of-week; `*`, lists, ranges, steps) in `automations/schedule.rs` with unit tests; do not add a dependency for it. `next_after(expr, from) -> Option<DateTime>` is the whole API.
- `missed` policy: if Castle was closed when a run was due, `skip` (default) or `run_once` on the next start. Never "catch up" every missed run.

### 4.2 `message` (phase 4)

- Fields: `{ "platform": "telegram|discord|whatsapp|any", "match": "contains|starts_with|regex", "text": "invoice", "case": false }`.
- Hooks into the bridges' incoming path, only for the one approved chat of each platform (the same gate as the companion chat). A matching message is **not** also sent to the companion unless the job says `"also_chat": true`. Intake trouble before any job starts (the database cannot be read, a limit is passed) never silences the chat: the message is answered as usual and the room shows the problem.
- The message text becomes `payload.text` (untrusted: see 8).
- Regex has a length cap and a match time cap.

### 4.3 `file` (phase 5)

- Fields: `{ "folder": "C:/path", "glob": "*.pdf", "event": "created|changed", "settle_s": 5 }`. Uses the `notify` crate if Castle does not already have a watcher (check `Cargo.toml` first); debounce by `settle_s` so a half-written file does not fire.
- Folder must be chosen with the folder picker; no network paths in v1.

### 4.4 `webhook` (phase 5)

- A tiny listener bound to `127.0.0.1` only (a fixed port range, default off), path `/hook/<id>/<secret>`; the secret is random, stored in Credential Manager, shown once with a Copy button. POST body (cap 64 KB) becomes `payload.body`. Rejects anything not from loopback unless the member later adds a tunnel themselves; Castle never opens the firewall.

### 4.5 `event` (phase 6)

Castle's own events: a reminder came due, a project task was ticked, a diary entry was saved, the memory backup finished or failed, the provider was switched, the database stopped. A fixed list in `automations/events.rs`; adding one is a one-line `emit` at the place it happens.

### 4.6 `manual`

Always available: **Run now** on every job, and `castle-memory automation run --name "..."` for a companion (see 9).

## 5. Actions

### 5.1 `companion` (phase 1)

Two modes, chosen per job:

- **`think`** (default, safe): `oneshot::run` with a new rules file `resources/automation-rules.md` ("you are running a scheduled job for the member; answer in plain words; you have no tools"). Castle itself reads the context the job asks for (read-only, from the database) and puts it in the prompt: `{{work_log}}`, `{{reminders}}`, `{{projects}}`, `{{today}}`, `{{payload.text}}`. The model never touches the database or files. Output is the run's `output`.
- **`session`** (opt-in, with a warning on the editor): types the prompt into the real main session via `send_and_await`, so the companion can use its memory and tools. Only allowed when the trigger is `schedule`, `manual` or `event` (never `message`/`webhook`/`file`: untrusted text must not drive a tool-using session). Shows a clear banner "this job lets {companion} use its tools".

Model/provider: follows Castle's provider switch unless the job names one (`cli`, optional `model`, validated by `oneshot::clean_model`).

### 5.2 `command` (phase 2) - the cron-job part

- Fields: `{ "program": "C:/Python314/python.exe", "args": ["C:/scripts/report.py", "--daily"], "cwd": "C:/scripts", "env": {"NAME":"value"}, "stdin": "none|payload" }`.
- **Allow-list**: a program may only be used if the member added its full path on Settings > Automations > "Programs jobs may run". A job pointing at anything else is refused at save and at run. Compare by canonical path; no shell (`cmd /c`, `powershell -Command` strings are not allowed unless that exact interpreter is on the list and the script is a file argument).
- `args` is a list, never one string; `{{payload.*}}` is **not** substituted into `program` or `args`. A payload reaches the program only through the env var `AUTOMATION_PAYLOAD` (JSON) or stdin, as the job chooses.
- Hidden window (`CREATE_NO_WINDOW`, as the other helpers), working folder must exist, timeout from policy, output captured up to the cap, exit code recorded, process tree killed on timeout (Windows job object or `taskkill /T`).
- No elevation, ever. A job needing admin is not supported.

### 5.3 `castle` (phase 3) - Castle's own moves

A fixed list: `backup_memory`, `export_markdown`, `add_reminder{title,due}`, `add_task{project,title}`, `check_database`, `wake_companion`, `save_diary_prompt` (types the "save diary" button into the session), `notify{text}`. Each is a function in `automations/castle_actions.rs` reusing the code the buttons already call.

### 5.4 `http` (phase 7)

GET/POST to a URL the member typed, optional header whose value comes from a named key in Credential Manager (new key names, listed in `KEYS`), body template with the same non-substitution rule for anything from a payload except an explicit `{{payload.text}}` in a JSON string value (escaped). Response body trimmed to the cap. https only unless the host is loopback.

## 6. Delivery

`delivery_json` is a list; each entry is one of:

- `{"kind":"home"}` - a notification on HOME and the Alerts sound the member already chose; the run is one line in the room.
- `{"kind":"telegram"|"discord"|"whatsapp"}` - the approved chat of that platform. **Phase 3 adds `bridges::notify(app, platform, text)`**: refuses if the bridge is off or no chat is approved, splits long text, rate-limited (policy `rate_per_hour`, default 6) so a loop cannot flood a phone.
- `{"kind":"project","name":"..."}` - appends the output as a note/task to a Castle project (through the existing project functions, not by writing files).
- `{"kind":"memory_note"}` - a dated note in the companion's memory through the existing `memory save` path, only for `companion` actions, only when the member ticks it.

`on_failure` deliveries are separate: by default a failed run always notifies HOME even with an empty delivery list, so failures are never silent.

## 7. Runner (phase 1)

`src-tauri/src/automations/` (new): `mod.rs` (state, commands, `start`), `schedule.rs` (cron, next-run), `store.rs` (SQL), `runner.rs` (the loop, concurrency, timeouts), `actions.rs`, `castle_actions.rs`, `deliver.rs`, `events.rs`.

- `automations::start(app)` is called from `lib.rs` setup beside the bridges. One thread, 15 s tick **while Castle is open**: finds enabled `schedule` jobs with `next_run_at <= now`, applies the missed policy, starts runs.
- Each run is its own thread. At most **2 runs at once** (setting), **one run per job at a time** (a job still running is `skipped` with a note), and a per-run timeout. Starting a run writes the `automation_runs` row first (`running`), so a crash leaves a visible half-run, which the next start marks `failed: Castle closed during the run`.
- Retries: 0 by default, max 3, with 1/5/15 minute backoff.
- Chains (phase 7): `policy.on_success_run` / `on_failure_run` name another job; depth cap 3; a job cannot be reached twice in one chain.
- While the database is stopped, the runner pauses and the room says so (same "Start" hint other rooms use); jobs do not run blind.

### 7.1 24/7 operation (phase 8, revised 22 November 2026)

Castle is meant to stay running continuously, so phase 8 uses the existing process and runner while hidden, minimized or locked; apply missed-run policy after interruptions; expose backend heartbeat/database/active-run health; clarify sign-in startup and actual Quit. The member's taskbar choice is preserved. A sleeping or powered-off PC cannot run jobs. No new scheduler task, Windows service or second worker is part of this phase.

#### Original optional proposal (deferred, not implemented)

The following earlier closed-process worker design is retained as history. It is not the approved phase8 scope:


Mirror the Sentinel: one system task `MemoryCore Castle\Automations` (`schtasks`, every 5 minutes, runs `memorycore-castle.exe --automations-tick`, no window, no model). It handles only `schedule` jobs whose action is `command` or `castle` and whose policy has `only_when_castle_open = false`, reading a snapshot `automations.json` that Castle rewrites on every change (the tick must not need the database, exactly like `--sentinel-tick`), writing results to a journal that Castle imports on open. Installing the task needs the member's yes in Settings; removing it removes everything. Companion jobs never run while Castle is closed.

## 8. Security rules (each of these has a test)

1. **Untrusted text never chooses a command.** Message, file name, webhook body and companion output are data. Tests: a message `"; calc"` or a file named `a&calc.pdf` reaches the program only via `AUTOMATION_PAYLOAD`/stdin; the args list is unchanged.
2. **Allow-list for programs**, canonical path compare, refused at save and again at run. No shell strings.
3. **Companion output is never executed.** A `companion` action returns text. The only thing text can do is be delivered or saved.
4. **Tool-using sessions only from trusted triggers** (`schedule`, `manual`, `event`), never `message`, `webhook`, `file`.
5. **Nothing is on by default**: the room, every new job and the webhook listener start off. A job proposed by the companion has `state='proposed'` and cannot run until the member approves it in the room.
6. **Secrets** live in Credential Manager; never in `trigger_json`/`action_json`/`output`. Run output is scrubbed of any stored secret value before it is saved.
7. **Rate limits** on outbound messages, on runs per hour per job, and on regex time.
8. **Kill switch** (`automations_paused`) is checked before every run start and every delivery.
9. **No elevation, no network listener except loopback**, no firewall changes.
10. **Audit**: every run, delivery and approval is a row; the member can read all of it in the room.
11. The member's words about a job (`note`) and the model's output are shown as text, never as HTML.

## 9. The companion and the CLI

- `castle-memory automation list|show --name|runs --name|run --name` (read and manual run) and `automation propose --name ... --trigger ... --action ... --file <json>` which creates a `proposed` job. Nothing else: a companion can **propose** and **run once** but cannot enable, edit an approved job or delete. (Same idea as `trait propose`.) Update `resources/castle-block*.md` so companions know the verbs.
- In the room, an "Describe what you want" box asks the companion (one-shot, `think`) to turn plain words into a draft job; the draft opens in the editor, unsaved, for the member to read and save.

## 10. The room (UI)

Follow `docs/settings-standard.md` and the Messaging room's card look; native controls themed; the room preloads behind the loader (never on arrival).

- Top: room title **AUTOMATIONS**, a **Pause all** switch, a "New job" button, "Describe what you want".
- List: one row per job - switch, name, trigger summary ("Every weekday at 08:00"), next run, last result pill (OK / Failed / Running / Skipped), **Run now**, menu (Edit, Duplicate, History, Archive).
- Editor (a sheet): steps in the order Trigger - Action - Delivery - Safety; every field has a plain label and a HelpTip; a **Test run** button that runs once with a sample payload and shows the output without delivering.
- History drawer: runs with status, duration, summary, expandable output, delivery results.
- Settings > Automations (new page, group "Connect"): allow-list editor with a path picker, concurrency, retention, webhook switch and port, the closed-Castle task install/remove.
- Empty state teaches with two ready-made examples (see 11) that the member can add with one click (added switched off).

## 11. Examples the first release should be able to express

1. **Morning briefing**: every weekday 08:00, `companion/think`, context `{{work_log}}` + `{{reminders}}`, prompt "what is open and what is due today, in five lines", delivery Telegram.
2. **Weekly backup check**: Sundays 20:00, `castle/backup_memory`, on failure Telegram.
3. **Invoice catcher**: message trigger (WhatsApp contains "invoice"), `castle/add_task` to project "Admin" with `payload.text` as the title, delivery HOME.
4. **Nightly script**: every day 02:00, `command` python `C:/scripts/report.py`, deliver output to a project note, failure to HOME.
5. **Downloads tidy**: file trigger on a folder, `command` with the file path in `AUTOMATION_PAYLOAD`.
6. **Session recap**: Fridays 17:00, `companion/session` with "save diary", trusted trigger, delivery HOME.
