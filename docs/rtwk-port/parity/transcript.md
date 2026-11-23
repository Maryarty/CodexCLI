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

---

# Chunk `chunk-transcript` (tier 2): state at the first handoff

Branch `worktree-agent-a78257323a3aeb7ab`, on `claude/tui-rtwk` at cabe0b853. Step 2's
sheet is done; step 2's /diff and steps 3-10 are not started. Nothing in the tree is red.

## Done

| step | what | where |
|---|---|---|
| 1 (part) | the 2 static snapshots live and 3 `kit::na` (ratatui renderable internals); the 18 transcript-overlay tests keep `kit::missing("the transcript sheet, step 9")` | `tests/oracle/pager.rs` |
| 2 (sheet) | `StaticPager`: the `/ ` title row, the text from `parse_ansi` in a `SingleChildScrollView` over a `~` filler (`LayoutBuilder` + `Stack`), the rule with ` N% ` (`ListenableBuilder` over the `ScrollController`), the hint rows, the Pager keys (up/down/k/j, pgup/pgdn/space/shift+space/ctrl+b/f, ctrl+u/d half pages rounded up, home/end, q/ctrl+c close, Enter = the action when the doc has one) | `src/pages/session/transcript/pager.rs`, strings in `transcript/strings.rs` |
| 2 (model) | the document is the model's: `transcript::Model.pager: Option<PagerDoc{title,text,action}>`, `Ev::Pager`, `Intent::ClosePager`; the fixture seed `{"kind":"pager",...}` runs through `apply` | `core/src/screens/transcript.rs` |
| 2 (entries) | debug entries `pager_basic`, `pager_wraps_long_lines`, `pager_long`, `pager_colors`, `pager_action` | `transcript/mod.rs`, `transcript/fixtures/*.jsonl` |
| tests | 11 page tests, the half-page rounding shown biting once: text at column 0, one-row scroll, pages, half pages, home/end, percent, wheel, q/ctrl+c close, the action key listed and bound only with an action, ANSI colours, hint clipping | `tests/pages/transcript_pager.rs` |

Oracle shards after this commit (`summary.py --run`; real pass / n/a / pending):
pager 2 / 3 / 18 (before 0 / 0 / 23), transcript_view_1 12 / 1 / 38, _2 5 / 7 / 40, _3 1 / 34 / 15,
_4 0 / 1 / 59, _5 1 / 9 / 49 (the last five as tier 1 left them). Checks that ran: `clippy
--all-targets -D warnings` exit 0, the 11 new page tests. Not re-run: whole nextest, `--workspace`.

A `Column` centres its children across by default (Flutter's `CrossAxisAlignment.center`): the
pager's text rows were centred in the width of the widest one until `CrossAxisAlignment::Stretch`.

## Location note (for popups)

The brief put the sheet in `src/sheets/pager/`; registering it needs `pub mod pager;` in
`src/sheets/mod.rs` (chunk-popups'). It is at `src/pages/session/transcript/pager.rs`
(`pub struct StaticPager`, a route widget: `Navigator` push, or a debug entry `EntryOpen::Route`).
Another chunk that opens it (approvals, provider details, /diff) sends `transcript::Ev::Pager(PagerDoc)`
through `apply`, then pushes `StaticPager`. Moving it under `sheets/` is a rename and one `mod` line.

## Needs from other owners (exact)

1. **chunk-live, `src/pages/session/session_page.rs`**: every keyboard behaviour of steps 3-8 needs it.
   A `ModeKeys` wrapper (to build in `transcript/mode.rs`: a `Focus` with the mode's bindings that is not
   handled when the controller has nothing to do) around `body`, inside the page's own `Focus`;
   `ChatMsg::Escape` and `Action::OpenTranscript`/`FindTranscript` forward to the controller
   (`w.ui.mode`) first; `detailed_transcript` and `esc_primed` become the controller's state. About
   30 lines, as the brief says.
2. **keymap, `core/src/app.rs`**: `Command::GitDiff{cwd,ticket}`, `Ev::GitDiff{ticket,Result<(bool,String),String>}`,
   `Intent::EditPrompt{thread,nth}`; `tests/kit/seed.rs`: `history_pages`, shift/right/triple click, key release.
3. **chunk-live, `core/src/threads.rs`** (and cells' `threads/user_prompt.rs`): the Revert command in
   ThreadFx and the ThreadReverted apply arm; `UserPrompt` keeps local/remote images and element placeholders.
4. **composer**: `ComposerDraft::restore(UserPrompt)`.
5. **commands**: `/diff` -> `Command::GitDiff` (today `Target::Pager`), the owned `/copy` branch.

## Left, in the brief's order

- Step 2 rest, /diff end to end: the pure argv builders of `get_git_diff.rs` into
  `core/src/screens/transcript/diff.rs` (rev-parse, fsmonitor probe, filter config, tracked diff, `ls-files
  --others`, per-file `--no-index`); the runner in `services/shell.rs`; `Target::Pager` in `open.rs` opens
  `StaticPager` when the doc arrives. The wiring waits on need 2; builders and runner can land first (no
  oracle test registers `get_git_diff.rs`).
- Step 4, selection and copy: `ItemTexts` says the text of an item equals the texts of the `Text`s it builds,
  joined by "\n". The pane's rows are the cells' widgets (gutters are `Text`s too), so `TranscriptTexts`
  needs a text per `Item` kind from the cells chunk, and a way to leave the gutters out of the payload.
  Settle with chunk-cells before building: a `fn texts(&Item) -> Vec<String>` beside each cell, tested to
  equal the rendered rows. The Markdown copy source is the brief's D2 / G-new row.
- Steps 3 and 5-10 as in the brief §6; the seam for the key-driven ones is need 1.

---

# Chunk `chunk-transcript` (tier 2): state at the second handoff

Rebased on `claude/tui-rtwk` 0eb53b241 (keymap steps 1-4). Steps 8 and 9 are in; steps 3-7, 10 and
the /diff end are not. The task claim was still held by the previous subagent of this session
("already claimed"); I worked in the same worktree as the re-grant said.

## Done since the first handoff

| step | what | where |
|---|---|---|
| 8 core | `Backtrack` (Idle / Primed / Browsing, `esc`, `begin_from_detailed`, `older`, `newer`, `confirm`, `interrupt`, `needs_history`), `Targets::of(&Transcript, older_history)`, `nth_user`, `revert_before_turn(turns, &ItemKey)` (persisted prompt with elements, images, skills, mentions), the edit's RPC pipeline in the thread slot (`ThreadPart`: read turns, resolve, `thread/revert`, await `ThreadReverted`; ADR-003: only `ThreadFx` sends) | `core/src/screens/backtrack.rs`, 19 tests in `core/tests/backtrack.rs` (`mod backtrack;` added to keymap's `core/tests/all.rs`) |
| 9 | `TranscriptSheet{follow, modes}`: the `/ T R A N S C R I P T` title row, the chat's detailed `TranscriptPane` in an `Expanded`, the status row (`New activity · ` + `⌃space select`), the two hint rows (`render_hints`), the pager's keys over the thread's `Follow`, the wheel, `q`/`ctrl+c` close | `src/pages/session/transcript/sheet.rs` (+ `pager.rs`: `header`, `bindings`, `hint_row`, `navigation` are `pub(super)`) |
| 9 (mode) | browsing prompts: Esc starts it, Esc ends it, Left/h and Right/l move, Enter chooses and closes; the status row becomes the first spelling of `prompt_navigation_footer` that fits (8 spellings, `LayoutBuilder` + `segmenter_of`); the hints add `→ to edit next · enter to edit message` while a prompt is marked | `sheet.rs`; the mode is `widgets/thread_ui.rs` `Modes` (Flutter's `ValueNotifier<Mode>` over core's `Backtrack`), also `Follow::jump_to_first`, `Follow::half_page` |
| entries | `sheet_basic` (prompt + reasoning summary + 40-row answer), `sheet_prompts` (three prompts) | `transcript/mod.rs`, `transcript/fixtures/` |
| tests | 11 page tests in `tests/pages/transcript_sheet.rs` (title row, reasoning shown, status and hint rows, scroll keys, wheel, close, Esc browse and end, the spelling at 8 widths, Enter chooses and closes, `q` while browsing); the Esc binding removed once: 4 of them failed, restored | `tests/pages/transcript_sheet.rs`, registered in `tests/pages.rs` |

The sheet is its own route widget (a debug entry opens it); it takes the mode keys itself in its `Focus`.
So no `ModeKeys` wrapper in `transcript/mode.rs` was built: need 1 of the first handoff shrinks to
"mount `TranscriptSheet{follow: w.ui.follow, modes: w.ui.modes}` when `detailed_transcript` is on (push it
as a route or put it in place of the body) and make `Action::OpenTranscript` toggle it". The primed Esc of
the chat uses the same `w.ui.modes.esc(..)` / core `Backtrack::esc` (one controller per thread).

## Departures

- Revert by `ItemKey`, not by the oracle's ordinal recount of visible prompts through the persisted turns
  (`backtrack_revert_before_turn_id`): a transcript item carries its server ids, so a stale ordinal cannot
  occur. No ADR is bypassed (ADR-003 holds: the pipeline is in the thread slot, one writer).
- The edit's mention bindings (`$skill`, `@path`) of a persisted prompt are carried as `Mention` data in
  `Prompt`; the composer binding itself is the composer's (need 4).

## Needs added

6. **chunk-live (`session_page.rs`, `threads.rs`)**: mount the sheet (above); register
   `ScreenThreadIntent::EditPrompt(Edit)` (the hand list in `core/src/screens/mod.rs`, popups' file) and call
   `backtrack::apply_thread_intent`; the `ThreadReverted` projection; the 10 s confirmation timer
   (`EDIT_NOT_CONFIRMED`); history re-hydration; composer restore. Enter in the sheet only closes it: the
   `Selection{thread,nth}` that `Modes::confirm` returns is not yet sent (the sheet has no handler for it;
   give `TranscriptSheet` an `on_edit` callback when the page mounts it).
7. **cells / chunk-live (`transcript_pane.rs`)**: `TranscriptPane` takes the highlighted prompt
   (`Option<ItemKey>` from `nth_user`) to mark it (`›`, reversed) and to scroll it into view
   (`ensure_entry_visible`); today browsing changes the status row and hints only, not the list.
8. **`NO_PREVIOUS_MESSAGE` notice**: `Esc::NoPrevious` is dropped by the sheet; the chat shows the info line.

## Left (not started)

- /diff end to end (`get_git_diff.rs`, 933 lines with the fsmonitor probe, filter config overrides and the
  `WorkspaceCommandExecutor`): builders in `core/src/screens/transcript/diff.rs`, runner in `services/shell.rs`.
- Steps 3 (disclosure focus), 5 (find), 6-7 (select/copy modes), 10; the sheet's pagination states
  (`Earlier messages available`, `Loading earlier messages…`, `Retry history`), `ctrl+space` select.
- The 18 `kit::missing` tests of `tests/oracle/pager.rs` keep their reason ("the transcript sheet, step 9");
  the sheet now exists but each needs the find / select / pagination state it asserts, so none is turned on.

## Checks at the second handoff (after the rebase on 0eb53b241)

- `fmt --check` clean; `clippy --workspace --all-targets -D warnings` no error (only the backend pin's warnings).
- Whole nextest: 1738 run, 1729 passed, 9 failed before the fix round below. After it the `rules` root has two
  reds: `layers::thread_rpcs_only_in_threads` (mine, see below) and `rules::r14_widget_state_fields_hold_no_screen_model_types`
  (`src/pages/startup/onboarding/signin/provider/gcp.rs:44`, a doc comment: not my area). Known reds
  `shell_agents::typing_a_word_asks_the_server_once` and the `screens` `startup::session::a_remembered_mode_opens_without_a_prompt`
  also failed in that whole run (the second is not on the known list and not in my area; not proven on the tip).
- The failures that were mine and are fixed: prose in core (`edit_failed`, now the typed `EditError`; the three
  prose consts are gone, `Esc::NoPrevious` says it), R13 (`before_turn: TurnId`), ADR-010 literals in the sheet
  (`hint_gap`, `hint_joiner` from the table), R15 in `pager.rs` (the rule is `Row[Expanded(Divider), label, ─]`,
  the header a clipped run from the table).
- **Red left, mine: `layers::thread_rpcs_only_in_threads`**: `core/src/screens/backtrack.rs` `wire` builds
  `ClientRequest::ThreadRead` (with turns). Fix (chunk-live, `core/src/threads.rs`): `ThreadAsk::ReadTurns{thread}`
  and `ThreadAsk::Revert{thread, before_turn: TurnId}` with their `wire`; `backtrack::Command` becomes
  `Command::Thread(ThreadAsk)` as `agents::Command` is, and `decode` stays here. ~15 lines; I did not touch the file.
- Oracle shards (`summary.py`; pass / n/a / pending / red): pager 2 / 3 / 18 / 0, transcript_view_1 12 / 1 / 38 / 0,
  _2 5 / 7 / 40 / 0, _3 1 / 34 / 15 / 0, _4 0 / 1 / 59 / 0, _5 1 / 9 / 49 / 0; `shards.py --check`: 0 files stale.
  The sheet adds page tests, not shard passes (the pager shard's 18 need find/select/pagination, above).
- Binary: `check.py --bin rtwk --bin oracle --only long-transcript`: PASS on both. The sheet is not reachable from
  Ctrl+T in the binary until `session_page.rs` mounts it (need 6; Ctrl+T still toggles the inline detailed view),
  so the walk of the sheet is the 11 page tests through the debug entries `sheet_basic` and `sheet_prompts`.

# Chunk `chunk-transcript-2` (tier 2): state at the third handoff

Rebased onto 225122339 (`land/live-c5`, chunk-live step 0). Commits `8a7c2d88f`, `3799188a3`, `b2e58f609` and the
chrome commit after them; the tm checkpoint "partial ready: b2e58f609" was written before the chrome step.

## Done since the second handoff

- **The layers red is cleared.** `core/src/screens/backtrack.rs` `wire` goes through `ThreadAsk::ReadTurns` /
  `ThreadAsk::Revert` (`ThreadAsk::wire`); `decode` stays here. `layers::thread_rpcs_only_in_threads` is green.
  The two types the r14 rule tripped on were renamed earlier (`EscStep`, `DraftRestore`).
- **The marked prompt of a browsing sheet** (second handoff need 7): `Pane{mark}` takes the selected prompt, its
  body is reversed (`DefaultTextStyle` REVERSE) and `ListController::ensure_index_visible` scrolls it into view.
  Three page tests (`browsing_reverses_the_body_of_the_marked_prompt_only`, `the_mark_goes_when_browsing_ends`,
  `a_marked_prompt_out_of_view_is_scrolled_into_view`).
- **The sheet's passive status row** (`footer_with_navigation`): away from the tail it says `end latest` (bold key,
  dim action; the news and the hints give way to it); at the tail `⌃space select`; the history text and the news
  go when the row is too wide (`LayoutBuilder` + `segmenter_of`, the oracle's fallbacks).
- **The pager shard**: 5 pass / 8 n/a / 10 pending (`kit::missing`, each with its reason), from 2 / 3 / 18.
  The 8 n/a are the inline-only rows (G1) and the oracle's own widget mechanics.
- **`/diff` builders and runner** (`get_git_diff.rs`): `core/src/screens/transcript/diff.rs` is a pure step
  machine (`Diff::start`, `feed`, `DiffStep::{Run, Done}`; failures typed as `diff::Failure`): `rev-parse`, the
  fsmonitor probes (3), hooks off, filter drivers blanked through `GIT_CONFIG_*`, the tracked diff and
  `ls-files --others` together, one `--no-index` diff per untracked file. 9 state tests in
  `core/tests/transcript_diff.rs` take their expected argv, env and limits from the oracle's tests.
  `services/shell.rs`: `WorkspaceExec` (one command in a directory) and `git_diff(exec, cwd)` drive the machine
  to its end; 2 tests.
- **Pager chrome, the person's decision (brief §7 D1)**: static pager = full chrome (title row, percent rule,
  hints; the pager shard's 5 passes); `TranscriptSheet` takes `chrome: Chrome`: `Overlay` (title row, status row,
  two hint rows, no percent rule: the inline `TranscriptOverlay` and the resume preview) or `Bare` (the transcript
  alone: the owned full-screen Ctrl+T and backtrack). Entry `sheet_owned`; 3 page tests, two shown biting
  (the `Bare` entry opened with `Overlay` fails both `the_owned_sheet_*` tests).

## Needs (exact; nothing here is mine to edit)

1. **chunk-live, `session_page.rs`**: Ctrl+T / Esc / F3 and the sheet mount. The owned full-screen app mounts
   `TranscriptSheet{follow: ui.follow, modes: ui.modes, chrome: Chrome::Bare}` (the chat's own footer then names the
   browsing keys: `prompt_navigation_footer` is chunk-status's); the resume preview mounts it with
   `Chrome::Overlay`. An `on_edit` callback for the `Selection{thread, nth}` that `Modes::confirm` returns.
2. **chunk-live, `threads.rs`**: the `ThreadEv::Reverted` arm forwards `backtrack::apply_thread(.., Reverted)`.
3. **chunk-live / host, `core/src/app.rs`**: `Command::GitDiff` / `Ev::GitDiff`, `Intent::EditPrompt`; the host's
   `WorkspaceExec` over `OneOffCommandExec`; `/diff` -> `Command::GitDiff` and the report into `Target::Pager`
   opening `StaticPager`. The words for `diff::Failure` are the consumer's table: the oracle says
   ``git {args:?} failed with status {status}`` for `Git`, the host's text for `Shell`.
4. **chunk-cells**: `ItemTexts` per item (steps 4-7: find, select, copy need a text per item).
5. **ThreadModel**: history Loading / Failed states and a feed route to a history page (the sheet's
   `Loading earlier messages…` / `Retry history` rows have no test route).
6. **chunk-live / composer**: the `NO_PREVIOUS_MESSAGE` notice, the 10 s `EDIT_NOT_CONFIRMED` timer, composer restore.

## Left

Steps 4-7 and 10 (find, select, copy, disclosure details): blocked on need 4 and on the mount (need 1); the 10
pager `missing` rows; the sheet's news / partial / Loading / Failed rows' tests (need 5).

## Rows for the lander

No new rtwk-gaps row. The tm area note: `tests/pages.rs` left this task's area when chunk-composer claimed it;
`tests/pages/transcript_sheet.rs` is registered through the root `tests/pages.rs` already.

## Checks at the third handoff

- `fmt --check` clean; `clippy --workspace --all-targets -D warnings` green; whole nextest 981 run, 981 passed
  (the `screens startup::session::a_remembered_mode_opens_without_a_prompt` timing test failed once in a loaded
  run and passes alone and in the full run: a flake, not on the known list); `shards.py --check`: 0 files stale.
- Oracle shards (`summary.py --run`; pass / n/a / pending / red): pager 5 / 8 / 10 / 0, transcript_view_1 12 / 1 / 38 / 0,
  _2 5 / 7 / 40 / 0, _3 1 / 34 / 15 / 0, _4 0 / 1 / 59 / 0, _5 1 / 9 / 49 / 0; total 24 / 60 / 211 / 0.
- Run it. `check.py --bin rtwk --bin oracle --only long-transcript`: PASS on both (`/tmp/ct2-cap`). The sheet is
  not reachable from Ctrl+T until `session_page.rs` mounts it (need 1), so I walked it in the real binary through
  `--debug-screens` (a pty driver with a small VT reader, 100x30): `sheet_basic` shows the title row, the transcript,
  `⌃space select` and the two hint rows; Home gives `end latest` and the hints give way; Esc gives the browsing
  status row and the two hint rows with `→ to edit next · enter to edit message`; `sheet_owned` shows only the
  transcript (no title, status or hint rows), Esc marks the prompt. The oracle has no debug menu and its Ctrl+T
  sheet is not in a mock-model scenario, so no row of the sheet was compared against the oracle binary: the rows the
  page tests assert are those of the oracle's `transcript.rs` render (`render`, `render_hints`), read, not run.

# Block `block-transcript`, child `clipboard-service`: the copy's destination and the HTML flavour

Commits `5bcedad69` (dependencies: `pulldown-cmark` with `html`, `url`; both were already in `Cargo.lock`) and
`bb35cf755`. `land-check` green on `bb35cf755` (`ALL fail=0`: lint-build, nextest, 39/41 scenarios, the two reds
are on `known-red.txt`).

## What was already there (found, not built)

The clipboard service of the task text had landed under other names before this child started: core
`Ask::Copy { text, format }` / `Done::Copy(Result<CopyStatus, String>)` (`core/src/app/effects.rs`), asked with
`Intent::Effect { request, ask }` and read back with `CodexApp::effect(request)`; `services::clipboard`
(routes in the oracle's order, the worker with the five second setup budget, `Busy`, the lease, tmux target,
OSC 52 check, the WSL writer, X11 PRIMARY, `Machine` over arboard) in `src/services/clipboard.rs` and
`clipboard/{machine,tmux,wsl}.rs`; the host answers in `services/effects.rs` (`Services::effect`), the OSC 52
sequence goes to the owner of the terminal (`Ran.terminal`). The words of the result are
`NoteKey::Copy { label, result }` (`threads/note.rs`, `pages/session/strings.rs`).

## Added

- `CopyDestination { Clipboard, Primary, ClipboardAndPrimary(Arc<str>) }` in core, and `Ask::Copy.destination`
  (oracle `worker::CopyDestination`, used by `selection_clipboard.rs`: copy-on-select publishes PRIMARY). The three
  existing callers pass `Clipboard`. A PRIMARY-only publication answers `Unconfirmed`, as the oracle's worker does.
- The HTML flavour of a Markdown copy: `services/clipboard/html.rs` `render_markdown` (oracle `clipboard_html.rs`:
  raw HTML escaped, images alt text only, a link that is not http(s) with a host stays as ` (destination)` text),
  and `format_of` offers it for `CopyFormat::Markdown` (of the text) and `MarkdownSelection(source)` (of the
  semantic source) to the native route. The four oracle tests are carried with their expected strings (the oracle's
  two snapshots written out).
- Tests: `html_is_offered_only_for_markdown`, `a_selection_publishes_primary_alone_or_with_the_clipboard`
  (`services/effects.rs`), `clipboard/html.rs` 5 tests.

## Departures and open rows

- `html.rs` calls `pages::session::cells::markdown::lift_markdown_fences` (the oracle's `unwrap_markdown_fences`
  of the transcript): services import a pages function. It is a pure string function; moving it to a neutral place
  (core, or `services`) is a rename and two `use` lines, left to the cells owner.
- The oracle's worker re-submits a PRIMARY publication that arrived while it was `Busy` once the worker is free
  (`worker.rs:178-182`, `:357`), tied to the selection's owner `Arc`. Here `Busy` is answered and the asker asks
  again; the selection's owner (select mode / mouse selection) holds that.
- `Ask::ClipboardText` has no source: the oracle's middle-click reads PRIMARY (`PasteSource::Primary`,
  `right_click_paste.rs`). The reader is `services/clipboard/read` (clipboard only); a `source` on the ask and the
  PRIMARY reader (`primary::read`, arboard `GetExtLinux`) come with the right-click paste, which has no consumer yet.
- No binary run: nothing in the app asks for a copy yet (`/copy` is `NotAvailable`,
  `core/src/app/session.rs:278`; the transcript has no selection copy). The first consumer is the next child.

## chunk-transcript-2: selection and copy (2026-11-23, branch fleet/block-transcript, commit 542959815, landed in 51abab645)
Shard counts at that commit: transcript_view_1 green 14 / pending 36 (chat-live) / n/a 1.
Own tests: `tests/pages/selection_copy.rs` (drag, release copy, right click, Ctrl+C / Cmd+C / Ctrl+Shift+C, Enter = copy and follow, Esc, footer hint at 4 widths, reversed highlight, feedback line, clear after Confirmed), `core/tests/selection.rs` (the Effect route), unit tests of `CopyLog` and `TextSelection`.

Decisions (ADR-013): the selection is drawn state in the session page (`SelectionHost` over rtwk `SelectionArea` and the `TextSelection` controller, Flutter's SelectableRegion); no `FeatureView` hook. Only the verb (`Intent::Effect` + `Ask::Copy`) and the host's `Done::Copy` cross into core (`CodexApp::latest_effect`); the UI's `CopyLog` follows it and draws the gap line.

Binary: scenario `text-100` (100x30 pty): answer on screen, drag over two words, right click, drag + Ctrl+C (kitty CSI 99;5u), Esc; both binaries, driver `/tmp/selprobe/drive_sel.py` (check.py has no mouse step; the oracle needs `--no-daemon`).
Equal: the highlight (SGR 7 in both after the fix), the footer hint during a selection, the line "Copied N chars to host clipboard" and its place, the selection gone after a confirmed copy, Esc clears.
Differing rows:
- Release cell: the oracle's range ends before the cell under the pointer (drag cols 26..37 copies "w0500 w0501", "Copied 11 chars"); rtwk `SelectionArea` includes it ("w0500 w0501 ", "Copied 12 chars"). rtwk gap; no workaround in our code.
- OSC 52: rtwk `SelectionArea` always queues its own OSC 52 on a release copy, so a release copy writes it twice (area and host effect); the oracle once. Known (rtwk task clipboard-provider).
- The status line "· Mock scenario" (oracle) is another page's row, not selection.
Gap rows: (1) `SelectionArea` drag end includes the release cell, the oracle's is exclusive. (2) `SelectionAreaController` has `select(target, item, start, end)` but no read of start/end: Shift+click and keyboard extend cannot be built.
Sweep: Markdown payload (`CopyFormat::MarkdownSelection` needs the item texts; plain text only), X11 primary selection, `mouse_scroll_speed`, link click, the oracle selection tests at 3-4 rows stay `kit::pending` (the bottom pane leaves the list 0 rows; chat-live), deselect compares text not range, the kit has no clipboard-answer step (`answer_effect` is used).
Mouse: right click copies (a Listener in `SelectionHost`); drag, double and triple click are `SelectionArea`'s.

## chunk-transcript-2-history-pages (2026-11-23, branch fleet/block-transcript, commit ee4c83bd9 merged with claude/tui-rtwk, landed in 51abab645)
Built:
- `transcript/history.rs` (new): the status row's spellings for the thread's history, widest first (oracle `footer.rs` `status_line_with_navigation` and the fallbacks of `footer`): `↑ Loading earlier messages… · esc latest`, short `↑ Loading… · esc latest` / `↑ … `, refused `Retry history: ⌥</⌃home.  esc latest` / `Retry: ⌥</⌃home` / `⌃home retry`, loadable `Earlier messages available.  `, selected and refused `… · Retry: ⌥</⌃home`. Read from `ThreadModel::history_state()` through `select`.
- `follow::footer` (chat) and `sheet::status` (transcript sheet) take their row from it; `sheet.rs` lost its `partial: bool`.
- `transcript_pane.rs`: a page joining above is no news (`tail` is the last row's key, not the row count); at the list's first line the row the reader was at is brought back to the top (`keep_first`, `jump_to_index_rows` skipping the separator the cell gains), and `find_index` lets the list keep a row elsewhere (Flutter's `findChildIndexCallback`).
- kit (`tests/kit/replies.rs`, tests only): `page_requests`, `pages_waiting`, `history_page`, `history_failed`, `agent_turn`, `at_the_top_of_a_partial_history(turns, Page::{Loading, Failed, Complete})`.
Tests: `tests/pages/history_pages.rs` 9 green (one request at open, named once away, loading at the top, once not per tick, shortened at 40, refused with its retry at 80 and 28, asked again at the top, a join keeps the rows and is no news). Oracle un-pended and green: cells_2 `wheel_at_hidden_session_header_keeps_the_loaded_message_anchor`, `session_information_waits_for_older_pages_and_reappears_after_completion`; transcript_view_1 `older_page_loading_uses_the_status_row_without_moving_content_or_cursor` (3 widths x Loading/Failed/Complete; LoadingBeginning reads as LoadingOlder in the snapshot); transcript_view_2 `loading_preserves_selected_content_and_copy_action`.
Shards after: cells_2 22/4/7, transcript_view_1 15/1/35, transcript_view_2 5/7/39 (+1 red below), _3 1/34/15, _4 0/1/59, _5 5/9/45.
Red not from this task: transcript_view_2 `ordinary_click_opens_bare_and_markdown_links_on_release` (no `docs` after the click) is red at the tip without this task's changes too.
Binary beside the oracle (`/tmp/selprobe/drive_hist.py`: 24 turns, resume, wheel to the top): the same final screen: header, prompt 00..02, "Back to bottom", footer `enter/esc latest · ? shortcuts`; the older page joins and the view ends at the top in both. Differing rows:
- rtwk shows a `Tip: Use the desktop app...` row above the composer at the tail (not this task).
- One wheel notch moves rtwk far more rows than the oracle (60 notches reach the top in rtwk, prompt 13 in the oracle).
- The Loading state lasts a frame on a local server: its text is the kit test's, not seen in the pty.
Needs:
- (seed.rs, claimed by another session) the kit does not consume the script's `reply` steps; the helper above answers pages from Rust instead. The oracle's find/select tests that join a page (transcript_view_3 `page_join_*`, _4 :188 ff, _5 :559 ff) want the same helper plus the features below.
- (chat keys, `session_page.rs`) the jump keys the footer names: `ctrl+home` / `alt+<` (walk to the beginning: scroll to the top and page until complete; the retry of a refused page) and `ctrl+end` / `alt+>` (latest). The retry is today the wheel at the top.
Sweep:
- transcript_view_3 `page_join_*` (:278, :306, :330, :351, :394, :431) are the oracle's reading-revision and group replacement; the port keys rows, a join keeps the reader's row (tested), the oracle's group merge has no counterpart.
- A join while the reader is 1-3 rows below the first line moves the view one row (the first row gains its separator); only the first-line case is tested.
