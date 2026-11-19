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
| Import (`/import`) | `PageRoute` push of an own `import::Screen` | step of StartupPage | to do |
| onboarding (S02-S04) | `onboarding::route()`, no caller (stub) | StartupPage | to do |
| startup (S05) | `show_dialog` stub | StartupPage step / dialog | not touched |
| Pager (`/diff`, Ctrl+T) | nothing (pending chat-live) | mode of SessionPage | not touched (chat-live) |
| Backtrack (Esc Esc) | `PageRoute` push of a stub | mode of the expanded transcript | not touched (needs chat-live's pager mode) |
| Pets (`/pets`) | `PageRoute` push (page) | modal sheet | to do |
| Voice | `PageRoute` push of a stub | modal sheet / widget | to do |
| Status, Select, Settings, Dialogs | `show_dialog` | sheets/dialogs | none |
| Approvals | `show_dialog` bottom pane (`pane_options`) | modal sheet | none |
| debug menu (F12) | `PageRoute` push | dev tool | none |

Note: the task description (brief) could not be read by this agent (`get_task`
was denied by the permission layer); the work follows the prompt and
architecture §1 Amendment 2026-11-19b only.
