# Contributing to MemoryCore Castle

Keep each pull request focused on one feature, fix, or documentation change. Start
from the current `main` branch and leave unrelated changes for another PR. If your
work depends on an open PR, link it and explain the dependency.

## Pull request titles

Use `type: short description` with an imperative verb:

- `feat: add an optional top-bar clock`
- `fix: verify companion presence before showing online`
- `docs: add contribution guidelines`
- `refactor: simplify provider selection`
- `test: cover companion admission failures`
- `chore: update build dependencies`

Describe the final change, not the history of the conversation or work session.

## Pull request descriptions

Use the [PR template](.github/pull_request_template.md) for every PR, including
drafts. Keep its four headings in this order; write `Not applicable` where a
section does not apply. A small change needs only a sentence or two per section.

1. **Problem**: explain the user-visible problem or missing capability. For bugs,
   include a concrete trigger and the observed behaviour. Link relevant issues.
2. **Changes**: describe the resulting behaviour and any important implementation
   choice. Keep this about the final diff.
3. **Validation**: record exact commands and completed results, including failures,
   skips, and relevant environment assumptions. Include screenshots or a short
   before/after description for visible UI changes.
4. **Risks and limitations**: list compatibility concerns, required configuration,
   dependencies, and checks that were not run. Write `None identified` when
   appropriate; do not imply that an unrun check passed.

Check the description against the final diff before requesting review. If the scope
changes, update both the title and description. Avoid raw logs and unsupported
claims that a failure is pre-existing; compare the same command and environment
on the base commit before making that claim.

## Validation

The Windows CI workflow runs:

```sh
npm ci
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

Run checks appropriate to the affected behaviour. For documentation-only changes,
review links and formatting and run `git diff --check`; application tests may be
marked not run with that reason. For code changes, include relevant regression
coverage and report which CI checks you ran locally and which remain unrun.

Separate clean-checkout tests from runs using a personal `.env`, saved credentials,
live services, or private helper scripts. Label local-only tooling explicitly so
reviewers know what they can reproduce from the repository. Only quote test totals
from a completed run.

For UI work, check keyboard access, focus, disabled and error states, and narrow
windows. Prefer native buttons, dropdowns, and scrollbars styled with Castle's
theme; keep controls sized to their content when a wider control is unnecessary.

A new room's own title is a plain `<header class="bar"><h2>NAME</h2></header>` - no
background, no border (see LIBRARY, APPEARANCE or THEME). Reserve the boxed
`<header class="top widget"><h3>NAME</h3></header>` form (STUDIO, MARKET) for a room
whose top area is itself a toolbar grouping several controls; it is not the default
room-title style. A `HelpTip` with `block` only lays out correctly inside a wrapper
that is itself `position: relative` (give that wrapper the `has-tip` class, as
Settings does) - without it, the bubble anchors to a distant ancestor instead and
can add a stray scrollbar far from where the `?` actually sits.

## How a room looks: the room standard

Every room is built from the same few pieces, so a new one looks like it was always part of
Castle. The working models are MESSAGING and AUTOMATIONS; copy them, do not invent a new look.
Settings pages follow their own kit in [`docs/settings-standard.md`](docs/settings-standard.md).

| Piece | Use | Do not |
|---|---|---|
| **Container** | `position: absolute; inset: 24px; display: flex; flex-direction: column; gap: 16px; overflow: auto`, with the themed scrollbar (`scrollbar-width: var(--scrollbar-width); scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track)`) | `inset: 0` with your own padding, a fixed width, a page-wide `h1` |
| **Title** | `<header class="bar"><h2>NAME</h2><span class="spacer"></span> ...room buttons... </header>`. The `h2` is `margin: 0; font-size: 13px; font-weight: 500; letter-spacing: 0.2em; color: var(--accent)`. Room buttons sit in the bar after the spacer | `h1`, 20px titles, the sub line inside the header, a boxed header (only STUDIO and MARKET, for a real toolbar) |
| **Lede** | One `<p class="lede">` under the bar: `margin: 0; max-width: 60em; color: var(--text-dim)`. One or two sentences on what the room is for | Paragraph hints scattered through the room |
| **Cards** | Add the class `castle-room-card` and `import '.../automations/roomcard.css'`. It gives the border (`var(--line)`), 18px radius, `var(--panel-background)`, 14px blur, accent hover and the faint top line. Add `live` for an active item | Copying the card CSS into your room, `--border`, hard-coded colours, a new radius or shadow |
| **Buttons** | Native `<button>`, themed: `padding: 8px 14px; border: 1px solid var(--line); border-radius: 10px; background: none; color: var(--text); font: inherit; cursor: pointer`. Hover: `border-color: color-mix(in srgb, var(--accent) 45%, var(--line))`. Disabled: `opacity: .45`. `button.primary` (one per view, the main action): `background: color-mix(in srgb, var(--accent) 25%, transparent)`. `button.danger`: `color: #ef9393` with a matching outline. Exclude `.setting-switch` and `.glass-select` from the rule | A button with no styling (it renders as a grey browser box), filled grey or white buttons, icon-only buttons without an `aria-label` |
| **Selects** | `GlassSelect` | Native `<select>` |
| **Switches** | `SettingSwitch` | Checkboxes styled by hand |
| **Fields** | `padding: 10px; border: 1px solid var(--line); border-radius: 8px; background: var(--panel-background); color: var(--text); font: inherit` and a focus ring: `outline: 2px solid var(--accent); outline-offset: 2px` on `:focus-visible` | Default browser inputs |
| **Section labels** inside a card or editor | `h3` at 11px, weight 500, `letter-spacing: 0.16em`, `color: var(--accent)` | Bold 16px headings |
| **Help** | `HelpTip` beside a label (see the `has-tip` note above) | Long grey paragraphs under every field |
| **Text** | Sentence case ("Run now", "Approved programs"); a button says its action; no setting keys, file names or ports in labels | Title Case Labels, "Enable...", "Toggle...", questions as labels |
| **Colour** | Theme variables only (`--text`, `--text-dim`, `--accent`, `--line`, `--panel-background`); the one fixed colour is the soft red `#ef9393` for errors and danger | Raw hex colours, a colour that disappears in a light or custom theme |

Before asking for review, open the room next to MESSAGING in at least two themes and check:
no default browser control is visible anywhere (including empty, error, editor and history
states); the title, lede and cards line up with MESSAGING; keyboard focus is visible;
narrow windows do not scroll sideways. Put a before/after screenshot in the PR.

## Loading: at the start-up screen, not on arrival

Castle has a start-up screen (see `MAKE-IT-YOURS.md`, "A start-up screen of your own"). Its purpose is
that **a room is simply there when the member opens it**. So:

- A room that reads data or pictures from disk or the network must not make the member wait for that
  when they open it. Read it at start-up instead, behind the start-up screen, and keep it for the run.
- Heavy work goes to the core, off the page's thread (`spawn_blocking`, or an `async` command), and is
  done in turns small enough that the window never freezes.
- Large files are not shown full size in a list: make small copies once, in the background, and keep
  them in Castle's own settings folder (STUDIO's `thumbs.rs` is the model).
- The room still reads again, quietly, when it opens or when something changes, and swaps in anything
  new; what is already loaded shows first.
- Start-up never waits forever: every piece has a time limit, and a piece that is slow or fails lets
  Castle open and finish its work in the background.
- The shared pieces to build on: `src/composables/useThumbs.js` (small copies of any picture),
  and one composable per room (`useStudio.js`, `useLibrary.js`, `useAppearance.js`, `useThemes.js`,
  `useWorld.js`), each reported as a start-up piece in `src/App.vue`.
- In the pull request, say what the room loads at start and how long that adds to a first and a later
  start. A new room that loads on arrival will be asked to move its loading to start-up.

## Privacy and scope

Never include credentials, personal memory, chat logs, private configuration, or
unredacted personal data in commits, screenshots, or PR descriptions. Use fixtures
for tests and redact screenshots. Do not bump the application version or include
generated installers unless the change explicitly concerns a release.

Keep replies to review findings concrete: verify the issue, fix it when needed,
and describe the validation performed. Merge and release remain separate from
submitting a PR.
