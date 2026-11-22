# transcript — parity

Task `chat-transcript` (Owned transcript behaviour), Tier 1: the transcript's
reader (follow), the "back to bottom" control, the row reserved above the
composer, the page keys, Esc/Enter back to the latest, the footer hints.
Oracle: codex-tui rust-v0.170.0 `transcript_view/follow_control.rs`,
`composer_gap.rs`, `footer.rs`, `input.rs`, `app/owned_transcript.rs`.
Step 4 (selection and copy, find, copy and select modes, disclosure) is not
started: those tests stay pending with their reasons.

## What was built

| piece | Flutter counterpart | where |
|---|---|---|
| the reader's controller, one per thread | `ScrollController` + `ChangeNotifier` (`at_tail`, `away`, `unseen`, `note_activity`, `jump_to_latest`, `page`) | `src/widgets/thread_ui.rs` (`Follow` in `ThreadUi`) |
| the list follows the controller; new tail while away = unseen; reaching the tail clears it | `ListView(controller:)` + a listener | `src/pages/session/transcript_pane.rs` |
| the row above the composer, the control in it while away | `ListenableBuilder` > `Center` > `MouseRegion` > `GestureDetector` > `Text`, the label from `LayoutBuilder` + `segmenter_of` | `src/pages/session/transcript/follow.rs` (`gap`, `FollowControl`) |
| the footer hint (`enter/esc latest`, `esc latest`, `New activity · esc latest`) | `ListenableBuilder` over the controller, `LayoutBuilder` picks the spelling | `transcript/follow.rs` (`footer`) |
| PageUp / PageDown wherever the caret is, unless the text opens the popup | `Shortcuts`/`Actions` at the page + the composer's key handler | `transcript/follow.rs` (`pager_key`), `session_page.rs` |
| strings | `strings.rs` of the group | `transcript/strings.rs` |

The bottom pane is one `Column` (`MainAxisSize::Min`): the blank row and the
gap row are `Flexible`, so a short window clips them first, as the oracle's
bottom pane (priority over the transcript, `desired_height().min(area)`) does.
No hand layout. The composer is keyed (`Key::value("composer")`): the status
row and the footer change kind around it, and without the key Flutter's
reconciliation made a new element and the composer lost its messages.

## Shards (oracle profile, `acargo nextest run --profile oracle --test oracle`)

| shard | tests | pass before | pass after |
|---|---|---|---|
| transcript_view_1 | 52 | 1 | 13 |
| transcript_view_2 | 52 | 7 | 12 |
| transcript_view_3 | 50 | 34 | 35 |
| transcript_view_4 | 60 | 1 | 1 |
| transcript_view_5 | 59 | 9 | 10 |
| total | 273 | 52 | 71 |

The whole oracle profile: 706 pass on this branch. The baseline list I kept
(665) was taken before the rebase onto the tip: 45 tests pass that did not (my 19
and 26 of other tasks' landings), 4 pass no more (below).

### The 19 stale pending markers removed (the tests passed under `pending`)

transcript_view_1: cancelling_owned_backtrack_returns_arrow_keys_to_the_composer,
composer_paste_releases_selection_and_retains_normal_cursor_movement,
copy_feedback_is_right_aligned_and_does_not_claim_terminal_delivery,
default_page_up_still_scrolls_the_transcript,
enter_preserves_search_backtrack_and_modal_ownership_while_scrolled,
escape_closes_selection_and_search_before_returning_to_latest,
footer_selection_defers_warning_clicks_and_preserves_the_draft,
fullscreen_composer_mouse_copy_and_input_ownership,
inline_confirmation_preserves_compact_viewport_without_replaying_history,
owned_drag_stops_when_focus_or_input_ownership_is_lost,
owned_transcript_keeps_text_out_of_the_pet_columns,
provider_error_escape_opens_resume_picker.
transcript_view_2: closing_find_at_bottom_does_not_show_back_to_bottom,
decimal_mouse_scroll_speeds_preserve_whole_rows,
find_and_shift_click_release_copy_payload,
height_changes_backfill_after_empty_click_but_preserve_deliberate_pause,
ordinary_click_opens_bare_and_markdown_links_on_release.
transcript_view_3: paused_header_reservation_survives_copy_until_scroll_or_resize.
transcript_view_5: startup_warning_keeps_first_visible_header_in_place.

Some of these pass because the thing they look for is absent (no selection, no
find yet): they bite when step 4 lands. `closing_find_at_bottom_does_not_show_back_to_bottom`
is vacuous for the same reason.

### Pending reasons rewritten (8)

The follow tests of the shards stay pending for what is not Tier 1, and now say so:

- G63 (`docs/rtwk-port/rtwk-gaps.md`: `MarkdownView` joins the single newlines
  of a paragraph, the oracle breaks the line): the fixtures' rows
  (`one\ntwo\n...`) never scroll in the port. `tests/pages/transcript_follow.rs`
  builds the same rows with hard breaks and tests the behaviour there.
- Kit limits: no bytes for `shift+enter` (no kitty keyboard protocol), plain-label
  snapshot sections (`before`, `Latest`, `Reading`) do not parse, the pager
  keymaps cannot be set in the config.
- View-level oracle tests: the oracle drives the transcript view alone (5, 6, 8
  rows); the chat screen has the gap row, composer and footer under it.
- A 1-row window: the bottom pane overflows a Column (RenderFlex overflow) where
  the oracle clips; the control itself hides at a zero-row viewport
  (`Follow::at_tail`).

No new gap row: G63 is the one that applies.

## Tests first

`tests/pages/transcript_follow.rs`, 22 tests, shown
failing before the code: label ladder at 80/20/17/12/9 (and with news), hidden
at the tail, the reserved row at the tail (last row + 3 = prompt), click returns
and keeps the draft, a click where the control is hidden reaches nothing,
PageUp/PageDown by a page less one row, Esc and Enter return, footer hints, news
named until the reader returns, news at the tail is not news, wheel reaching the
end hides the control, Esc on the empty composer returns and primes the
backtrack, page keys with a multiline draft are the transcript's and with the
popup open the popup's, a press on the control released elsewhere returns
nothing, the hover is reversed and bold, a window with no row for the transcript
has no reader away, the composer is the same element while the turn starts.

## Checks that said something

- Whole nextest on this branch: 814 of 820. Reds: `settings::esc_goes_back...`,
  `settings::enter_on_reset_all_memories...` (known red), and three that are
  flakes under load (30+): `settings::space_flips_a_switch_enter_saves_both_and_closes`
  (2 of 3 alone, timing of `h.advance(600)`), `settings::enter_on_the_reset_row_...`
  and `startup::session::fork_by_id_opens_the_copy` (pass alone). Not mine, not
  in known-red.txt: please add or fix (owner: settings porter).
- `skeleton::prompt_streams_the_answer_as_plain_text` was mine: the golden
  `tests/snapshots/chat/answer.snap` had one blank row between the answer and the
  prompt, the oracle has two (`app/snapshots/..._conversation_in_offset_rect.snap`:
  the gap row, the composer's own blank row). Golden updated, commit 3da67b163.
- `rules`: 66 of 66 (R14: the pane keeps a fingerprint of the tail item, not a copy
  of the core `Item`).
- fmt, `clippy --workspace --all-targets -D warnings` (exit 0), `shards.py --check`
  (55 shards, 0 files stale).

### Regressions outside my area (the oracle shards of another owner)

Four tests that passed on the base fail now, only because the reserved row takes one
row more of a 32x8 / short window; the oracle's own layout reserves it too (its
cells are rendered alone). Fix: one row more in the test's `.size(w, h)`.
Owner: chat-live-shards (`tests/oracle/cells_1.rs`, `cells_2.rs`).
- cells_1::ui_snapshot_apply_multiple_files_block
- cells_2::finalized_plan_file_citation_renders_as_local_path_snapshot
- cells_2::viewed_image_narrow_summary
- cells_2::viewed_image_retains_original_path_in_details

## Run it (real binary, `long-transcript`, both binaries, 120x40)

`check.py --bin rtwk --bin oracle --only long-transcript --capture-dir /tmp/chat-transcript-cap`:
2 of 2 PASS. Walked with a pty driver (keys and the SGR mouse, both binaries; the
driver is a script in /tmp, not in the tree):

| step | oracle | rtwk |
|---|---|---|
| PageUp from the tail | rows scroll a page less one; the control `↓ Back to bottom · esc` at column 50, the row two above the prompt; footer `enter/esc latest · ? shortcuts` | the same rows and column; footer `enter/esc latest` |
| typing `draft` | footer `esc latest` | `esc latest` |
| Esc | back at the latest, the draft kept | the same |
| wheel up x3, then a click on the control | control shown, the click returns, the draft kept | the same |

Differing rows of the `turn-1` checkpoint (the tail, `compare-transcript`): all 40 rows
are shifted by one row (869 of 4800 cells) because the oracle's footer is three rows
(the status line, the thread title `scenario`, the keys with `⚠ 1 warning · f2 to view`)
and ours two: owner chat-status. The structure above the composer is equal: `Worked for
Ns • HH:MM` (the row exists in both, the colour differs: `dim indexed(245)` against
`dim`), one blank row, the composer's surface row, the prompt. The bullets are
`indexed(116)` in the oracle and `brightBlue` here (cells).

### Delta found by the walk, fixed

The oracle appends ` · ? shortcuts` to the return hint while the composer is empty and
it fits (`bottom_pane/chat_composer/footer_state.rs` `render_transcript_footer`). Now the
footer does the same (`follow.rs` footer, two strings), test
`the_empty_composer_adds_its_shortcuts_key_to_the_footer_while_it_fits` (80 columns: shown;
31: absent; 32: shown; red without the code, once). After it: `pages` + `rules` 188 of 188,
clippy `--lib --test pages` clean, fmt clean. The whole nextest, the oracle profile and
`clippy --workspace --all-targets` ran before this last change (the disk filled up twice
during the final run: `No space left on device` from other builds); it touches only the
footer's text for the away reader.

## `session_page.rs` edits (for chat-live's carve)

1. `use super::transcript;`
2. `ChatMsg::Page(i8)` and its arm: `w.ui.follow.page(i64::from(pages))`.
3. The composer's key handler calls `transcript::follow::pager_key` before the field.
4. Submit arm: Enter on an empty composer calls `w.ui.follow.jump_to_latest()`.
5. Escape arm: away and not read-only: `jump_to_latest`, and the Escape is spent unless
   the draft is empty and nothing can be interrupted (then the backtrack primes as before).
6. `TranscriptPane(w.ui.follow.clone(), self.detailed_transcript)`.
7. The footer is wrapped by `transcript::follow::footer` for `FooterLine::Keys` only.
8. `status_above` (the working line or previews are above; page level, from
   `run.word(..) != "Ready"` or `Session::Starting`, not read-only). The slots that chat-live
   carves know this too: the predicate should move next to them.
9. The bottom pane `body` is a `Column(Min)`: status slot, pending input, notice, panel,
   backend control and banners, `transcript::follow::gap(..)` (non-read-only), the keyed
   composer, status line, footer, titles; the outer `Column[Expanded(1, transcript), body]`.
10. Focus bindings PageUp/PageDown to `ChatMsg::Page(-1/1)`.

App-level (core/app): nothing touched.

## Not done (step 4 and the rest)

Selection and copy, find, copy and select modes, disclosure focus, the pager (Ctrl+T)
stay pending in the shards with their reasons; the footer for selection and search is
the oracle's `footer.rs` rows still to build on the same `footer()` seam.
