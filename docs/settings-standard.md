# The Settings standard

How every Settings page is built, so anything new looks and behaves like the rest.
The pieces live in `src/components/settings/`.

## Anatomy: four levels, always in this order

| Level | Component | Rule |
|---|---|---|
| Page | `SettingsPage` | One per sidebar item. A title and one subtitle line saying what lives there. The sidebar sits in five fixed groups: You · Home · Connect · Data · App (`settingsnav.js`). |
| Group | `SettingGroup` | A card with a title, 2 to 7 rows. More than 7 means split it. A page has no rows outside a group. |
| Row | `SettingRow` | Label on the left, control on the right. Sentence case, 5 words or fewer. Optional `tip` (a HelpTip) beside the label. |
| Description | `description` prop | One line, 90 characters at most. What changes, not how it works. Anything longer goes in the tip. |
| Control | the kit below | Pick from the kit. No raw checkboxes, `.row` or `.setting-row` of your own. |

## The kit

| Control | Component | Saves |
|---|---|---|
| Toggle | `SettingSwitch` | at once |
| Choice, 2 to 4 options | `SettingChoice` | at once |
| Select, 5+ options | `GlassSelect` | at once |
| Text | `SettingText` | Save · Cancel, shown only while it differs |
| Secret key | masked value + Show · Test · Replace · Remove | Save · Cancel |
| Status | `SettingPill` + one fix button ("Check again") | read-only |
| Danger | `SettingConfirm` ("Sure?" with Keep, in place) | on the second press |

## Behaviour

- **Saving.** Instant controls save through the page's `saveRow(id, patch)` and flash "Saved ✓" for 1.5 s (`rowstatus.js`). No page-wide Save button.
- **Failure.** The control keeps its old value (the page takes new settings only once Castle kept them) and a red line under the row says what happened. No toast for a row's own error.
- **Depends on another setting.** The row stays visible and `disabled`; its description gives the reason and, when it helps, links to the page that fixes it. Never hide it.
- **Defaults.** Every plain setting's default is in `settingsDefaults.js`, read with `valueOf(settings, key)`. A test holds it to `Settings::default` in `src-tauri/src/settings.rs`.
- **Deep links.** Every group and row has an `id`. `openSettings('chat#pictures')` opens the page, scrolls to that row or group and lights it for a moment. Use it from other rooms instead of "go to Settings".
- **Danger.** Destructive actions ask in place with `SettingConfirm`, never `confirm()`. A page's Danger zone group (`danger`) is always last.
- **Big panels** (Widgets, Database, Backup) sit on their own page, inside a `free` group.
- **Windows and Mac.** Words for the computer's own parts come from `isMac`, never hard-coded.

## Words

Do: a label says what it is ("Moving wallpapers"); a description says what changes for the member; name the companion with `companionLabel`; buttons say the action ("Check again", "Replace", "Add one in Keys").

Don't: paragraph hints under a row; "Enable…", "Toggle…", "Configure…" in a label; setting keys, file names or ports in a label; questions as labels; Title Case Labels.

## Adding a setting

1. Find its page on the sidebar map. If it really fits none, ask before adding a page.
2. Put it in an existing group, or start a new group of at least 2 rows.
3. Pick the control from the kit. Need something new? Add it to the kit first.
4. Write the label (5 words or fewer) and at most one description line. More goes in the tip.
5. Add the default to `settingsDefaults.js`, and the field on the Rust side.
6. Give the row an `id`, and deep-link to it from the room that needs it.
7. Tests: the default, the save, the failure. Then reach it from the sidebar, change it, reopen Settings: still set.
8. Screenshot the row in the PR next to its group.
