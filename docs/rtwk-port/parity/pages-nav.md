# pages-nav — parity and route table

Task `pages-nav`: lay the app's navigation out as architecture §1 "Amendment
2026-11-19b" says (pages, modes, modal sheets, widgets). Branch
`tui-rtwk/pages-nav` from `claude/tui-rtwk` f70cbe9c6.

## Inventory (before): how each chat target / entry opens, from `src/screens/chat/open.rs`

| target / entry | opened as (before) | class by amendment 2026-11-19b | change |
|---|---|---|---|
| chat (home) | Navigator home (`ui/view.rs`) | SessionPage | none |
| Agents (`/agents`, `codex-rtwk agents`) | `PageRoute` push; `standalone` home with `start_agents` | AgentsPage | `Footer::Hints` on `page_keys::footer`; the notice replaces it as before | done: the oracle rows of tests/agents.rs hold (`live_center_navigation_and_complete_hints`, `live_center_hints_follow_configured_bindings`, widths one to ten) |
| ResumePage | stays on `resume/fit.rs` under gap G41 (probe `tests/resume.rs::g41_probe_key_hints_bar_has_no_priority`: no wide/compact spelling, no priority drop); keys not built yet (tab, left/right, ctrl+o/t/e) open the popup: `resume_footer_keys_are_bound` | done / G41 |
| UsagePage | two rows on `page_keys::footer` / `footer_with(" · ")`, rows equal to the oracle (tests/usage.rs summaries pass); tab cycles the reports; z, ?, R open the popup: `usage_footer_keys_are_bound` | done |
| StartupPage | its steps are stubs (mock-pre), no oracle footer rows to match; Esc closes the import step | left for mock-pre |

`page_keys::footer(keys, ..)` takes the oracle's key texts (`esc`, `↑/↓`,
`←/→`, `⌃c`, `ctrl + t`, `1–6`) and maps them to the keys the bar names, with
a label map back, so the bar shows them as the oracle writes them.

rtwk finding: `KeyHintsBar` fed by bindings drops a plain character key while
a text field has the focus, so `?` is shown with `from_pairs`; the `?`
binding itself is then swallowed by the composer (the oracle fires it on an
empty composer only): the binding exists, the popup for it is unreachable by
keyboard until the composer hands an empty-composer `?` up.
