# pages-nav — parity and route table

Task `pages-nav`: lay the app's navigation out as architecture §1 "Amendment
2026-11-19b" says (pages, modes, modal sheets, widgets). Branch
`tui-rtwk/pages-nav` from `claude/tui-rtwk` f70cbe9c6.

## Inventory (before): how each chat target / entry opens, from `src/screens/chat/open.rs`

| target / entry | opened as (before) | class by amendment 2026-11-19b | change |
|---|---|---|---|
| chat (home) | Navigator home (`ui/view.rs`) | SessionPage | none |
| Agents (`/agents`, `codex-rtwk agents`) | `PageRoute` push; `standalone` home with `start_agents` | AgentsPage | `Footer::Hints` on `page_keys::footer`; the notice replaces it as before | done: the oracle rows of tests/agents.rs hold (`live_center_navigation_and_complete_hints`, `live_center_hints_follow_configured_bindings`, widths one to ten) |
| ResumePage | hint rows on `page_keys::footer_fit` (a `LayoutBuilder` over `KeyHintsBar` variants in priority order: wide from 120 cells, compact, then the lowest priorities, then keys alone; `resume/chrome.rs` builds them; `resume/fit.rs` deleted; gap G41 closed as "not a gap"), the oracle rows of tests/resume.rs hold; keys not built yet (tab, left/right, ctrl+o/t/e) open the popup: `resume_footer_keys_are_bound` | done |
| UsagePage | two rows on `page_keys::footer` / `footer_with(" · ")`, rows equal to the oracle (tests/usage.rs summaries pass); tab cycles the reports; z, ?, R open the popup: `usage_footer_keys_are_bound` | done |
| StartupPage | its steps are stubs (mock-pre), no oracle footer rows to match; Esc closes the import step | left for mock-pre |

`page_keys::footer(keys, ..)` takes the oracle's key texts (`esc`, `↑/↓`,
`←/→`, `⌃c`, `ctrl + t`, `1–6`) and maps them to the keys the bar names, with
a label map back, so the bar shows them as the oracle writes them.

`?` on the session page (the person: not an rtwk gap, decided at the field):
`rtwk::editing::EditingKeys` was tried first (`TakeKey` = `Fn(&controller, &key,
KeyCx) -> bool`): a taken key is consumed by the field (the page's `?` binding
never sees it) and the closure has no `Emit`/`Cx`, so it cannot open the popup
itself. The decision is made where the field reports the edit instead: the
composer's `on_changed` (`ChatMsg::Changed`): the text is exactly `?` (typed
into an empty composer) -> the character is cleared and the shortcuts popup
opens; `a?` types the character. Test:
`navigation::question_mark_is_the_shortcut_on_an_empty_composer_only`.
(If `EditingKeys` should be the door: a `TakeKey` variant that can emit a
`Handler`, or a taken key that bubbles to the ancestors' bindings.)
