# pages-nav — parity and route table

Task `pages-nav`: lay the app's navigation out as architecture §1 "Amendment
2026-11-19b" says (pages, modes, modal sheets, widgets). Branch
`tui-rtwk/pages-nav` from `claude/tui-rtwk` f70cbe9c6.

## Inventory (before): how each chat target / entry opens, from `src/screens/chat/open.rs`

| target / entry | opened as (before) | class by amendment 2026-11-19b | change |
|---|---|---|---|
| chat (home) | Navigator home (`ui/view.rs`) | SessionPage | none |
| Agents (`/agents`, `codex-rtwk agents`) | `PageRoute` push; `standalone` home with `start_agents` | AgentsPage | none |
| Resume (`/resume`) | `PageRoute` push | ResumePage | none |
| Usage (`/usage`) | `PageRoute` push | UsagePage | none |
| Import (`/import`) | `PageRoute` push of an own `import::Screen` | step of StartupPage | done |
| onboarding (S02-S04) | `onboarding::route()`, no caller (stub) | StartupPage | done |
| startup (S05) | `show_dialog` stub | StartupPage step / dialog | not touched |
| Pager (`/diff`, Ctrl+T) | nothing (pending chat-live) | mode of SessionPage | not touched (chat-live) |
| Backtrack (Esc Esc) | `PageRoute` push of a stub | mode of the expanded transcript | not touched (needs chat-live's pager mode) |
| Pets (`/pets`) | `PageRoute` push (page) | modal sheet | done |
| Voice (F8) | `PageRoute` push of a stub | modal sheet (stub); the strip itself is a widget of mock-chat | done |
| Status, Select, Settings, Dialogs | `show_dialog` | sheets/dialogs | none |
| Approvals | `show_dialog` bottom pane (`pane_options`) | modal sheet | none |
| debug menu (F12) | `PageRoute` push | dev tool | none |

Note: the task description (brief) could not be read by this agent (`get_task`
was denied by the permission layer); the work follows the prompt and
architecture §1 Amendment 2026-11-19b only.

## Route table after

| target / entry | opened as (after) | where |
|---|---|---|
| chat | Navigator home = SessionPage | `ui/view.rs` |
| Agents | `PageRoute` push = AgentsPage | `agents::route` |
| Resume | `PageRoute` push = ResumePage | `resume::route` |
| Usage | `PageRoute` push = UsagePage | `usage::route` |
| Import | `PageRoute` push = `StartupPage { step: Import }` (body: `import::Screen`) | `onboarding::route(Step::Import)`, `import::route` |
| onboarding | `StartupPage { step }`, steps SignIn, Trust, Migration, Import (step controller = the page's state) | `onboarding::{StartupPage, Step, route}` |
| Pets | `show_dialog`, bottom edge, no frame, no barrier, no fade (`approvals::pane_options`, Esc left to the picker; the barrier closes the unseeded stub) | `pets::open` |
| Voice | `show_dialog`, same options, barrier closes (stub) | `voice::open` |
| Backtrack | unchanged: `PageRoute` push of a stub; it becomes a mode of the expanded transcript with chat-live | `chat/open.rs` |
| Pager | unchanged: nothing (chat-live) | `chat/open.rs` |
| Status, Select, Settings, Dialogs, Approvals | `show_dialog`, unchanged | |

## Tests

- `tests/navigation.rs::pets_is_a_sheet_over_the_session_page` (red in
  852e0b730, green in 9b986ca32): the transcript stays on screen while the
  pets sheet is up.
- `tests/navigation.rs::import_is_a_step_of_the_startup_page` (red: does not
  compile before the page exists).
- `tests/skeleton.rs::debug_menu_scrolls_to_the_cursor`: now waits for the
  sheet's fade-in (the only edit to an existing test).
- Existing `chat::transitions` tests for `/pets`, `/pet`, F8 and `/import`
  hold unchanged.

## Left / for others

- Backtrack and Pager become modes of the SessionPage's expanded transcript
  in chat-live; not done here (no pager widget exists yet).
- `startup::open` (S05 prompts) is still a dialog stub; `mock-pre` decides
  whether it joins the StartupPage steps.
- `approvals::pane_options` is the sheet options helper reused by pets and
  voice; a shared `screens::sheet_options` belongs in `screens/mod.rs`
  (outside this area).
- The brief (task description) was unreadable for this agent.

## Footers and the not-implemented popup (scope added 2026-11-19)

Shared: `src/ui/page_keys.rs` (`footer(pairs, key_style, hint_style)` on rtwk
`KeyHintsBar::from_pairs`; `not_yet(cx, action)` popup "Not implemented yet:
<action>", centred, Esc/Enter close); strings in `l10n/strings.rs` (`Pages`).

| page | footer | state |
|---|---|---|
| SessionPage | oracle idle footer `  ? for shortcuts` (`footer_shortcuts_default`); `?` bound via `KeyBindings`; Ctrl+T, F3 and `/diff` open the popup | done; the right-hand `100% context left` is not drawn (no context data in the model) |
| AgentsPage | hand-made hint row (`agents/view.rs` `hint_line`), keys bound via `KeyBindings` (`agents/mod.rs:1048`) | not moved to the shared footer; per-key popup audit left |
| ResumePage | hand-made width-fitted hint row (`resume/fit.rs`, oracle `hint_line_for_row`); keys bound via `KeyBindings` (`resume/mod.rs:423`) | left: KeyHintsBar has no priority collapse, probe against the oracle's rows before deciding (possible gap) |
| UsagePage | `strings.hints` line (`usage/mod.rs:396`) | left |
| StartupPage | stubs, no oracle footer text yet | left |

rtwk finding: `KeyHintsBar` fed by bindings drops a plain character key while
a text field has the focus, so `?` is shown with `from_pairs`; the `?`
binding itself is then swallowed by the composer (the oracle fires it on an
empty composer only): the binding exists, the popup for it is unreachable by
keyboard until the composer hands an empty-composer `?` up.
