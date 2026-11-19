# rework-dialogs — parity

Slice: the dialogs' rework on rtwk's standard means (approvals, select,
status card, shortcuts, feedback, settings, popovers, pets). Oracle:
codex-tui `bottom_pane/request_user_input`, `list_selection_view.rs`,
`status/`, `pets/picker.rs`. Tests: `tests/oracle/approvals_1.rs`,
`approvals_2.rs`, `chat_popups_2.rs` (shards), `tests/approvals.rs`,
`tests/pets.rs`, `tests/status.rs`, `tests/rules.rs`.

## Done

| file | route | tests |
|---|---|---|
| `approvals/dialog.rs` | `Dialog::bare`; placed by `show_dialog` options (`approvals::pane_options`: BOTTOM_CENTER, `barrier_alpha` 0, no fade) for the presenter and the debug menu; option rows an rtwk `Table` (mark column fixed(2), label `content`, description `fr(1)`) | oracle shards approvals_1 `permissions_prompt_snapshot`, approvals_2 `request_user_input_options_snapshot` pass; `tests/approvals.rs` 5 un-ignored |
| `dialogs/shortcuts.rs` | `Grid` of content tracks | `tests/dialogs.rs` shortcuts snapshot |
| `status/card.rs` | `Grid` `[content, fr(1)]`, column gap 2 | `tests/status.rs` `status_snapshot_includes_reasoning_details` |
| `pets/mod.rs` + core `pets.rs` | `SelectionController<usize>` cursor, `TextEditingController` query, `SelectionList` in an 8-row `SizedBox`, `ScrollNotification` for the "more below" row, preview status centred by `Align`; core keeps pets, current, preview and the intents `Highlight(selector)`, `Pick(selector)`, `Close` (no cursor, query or `Move`/`Search`) | `tests/pets.rs` 6 pass (the oracle's `slash_pets_picker` snapshot among them) |
| `ui/debug_menu.rs`, `ui/presenter.rs` | one `pane_options()` for both | approvals tests |

## Rtwk gaps found (rtwk-gaps.md)

| row | what | test |
|---|---|---|
| G39 | no content-sized selectable list with a shared label column and wrapped variable-height description cells capped to the room (Table fills the room, its intrinsic height is 0; Grid has no cursor) | pending(G39): approvals_2 `request_user_input_{long_option_text,tight_height,scroll_options,hidden_options_footer}_snapshot`, chat_popups_2 `approvals_selection_popup_snapshot` |
| G40 | no gauge of whole `█`/`░` cells | `tests/status.rs::limit_bar_is_a_rtwk_gauge_of_whole_cells`, ignored as pending(G40) |
| G35 (existing row) | `SelectionList` cannot scroll to a cursor moved through its controller: pets picker cursor can leave its 8-row window when the arrows pass the end | `tests/pets.rs::the_cursor_row_stays_in_the_window_when_the_arrows_pass_its_end`, ignored as pending(G35) |

## Not done (left in R15_NOT_YET / not started)

`select/dialog.rs` and `dialogs/feedback.rs` (G39: a `SizedBox` of a computed
row count, one line per row), `status/format.rs` (G40, `"█".repeat`), core
`screens/settings.rs`, `popovers.rs`, `commands.rs` (cursor/query/tab state
still in core; target `SelectionController`, `TextEditingController`,
`TabBar`).

## Deltas

None accepted. The G39/G40/G35 renderings differ from the oracle (rows one
line instead of wrapped; no unfilled glyph; a cursor off the window) and are
the rows above; no test is loosened.
