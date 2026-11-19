# session-shortcuts parity pack

Oracle: `bottom_pane/shortcut_overlay.rs` (`lines`, `close_hint`), `shortcut_help.rs`
(`group_lines`), `chat_composer/agents_navigation.rs`, `status_line_style.rs`,
snapshots `footer_mode_shortcut_overlay`, `agents_navigation_enabled`.

| oracle | tui-rtwk test | state |
|---|---|---|
| footer_mode_shortcut_overlay (100 wide, composer row skipped) | tests/dialogs.rs `session_question_mark_opens_the_footer_mode_shortcut_overlay` | ported |
| `group_lines` 2 / 1 column fallbacks | tests/dialogs.rs `session_shortcut_overlay_narrows_to_two_columns_and_to_one` | ported |
| `?` / Esc close | tests/dialogs.rs `session_shortcut_overlay_closes_on_question_mark_and_on_escape`, tests/navigation.rs `question_mark_is_the_shortcut_on_an_empty_composer_only` | ported |
| footer `← for agents · ? for shortcuts` | tests/navigation.rs `the_session_page_footer_is_the_oracle_idle_footer`, chat snapshots empty/answer | ported |
| `handle_agents_navigation` (Left on empty prompt) | tests/navigation.rs `left_on_an_empty_composer_opens_the_agents_page` | ported (FocusNode on_key_event; G43 closed, not a gap) |
| status_line_style.rs tests (segments, separators, colours off, PR link, none when empty) | tests/chat.rs `status_line::*` | ported (style: accent -> theme info/success/accent tokens) |

Left open (data the core model does not hold; owner session-core): status line items
and the saved item list (model+effort, cwd, branch, approval mode, context left/used,
5h/weekly limits), the footer's right side (`N% context left`, `warnings · f2 to view`
and the f2 binding to the warnings dialog). `status_items()` in `screens/chat/home.rs`
returns an empty list: nothing is invented. Not ported: thread-name colour
(`thread_color`) and `soften_status_line_color` (theme colours stand in).
Visual deltas: none. rtwk gaps: none (G43 was not a gap).
