# Brief: transcript chunk (owned transcript after tier 1, pager, backtrack; tm `pager`, `backtrack`, "Owned transcript behaviour")

Recon 2026-11-22 (recon-transcript, read-only, no cargo). Paths: `codex-rs/tui-rtwk` (ours), `codex-rs/tui/src` (oracle),
rtwk pin `~/.cache/codexcli/rtwk-pin-5001` (5001d2cd9; ADR-029: `selection_text` is the twk shape, ItemTexts removed). Tier 1 is the chat-transcript
porter's worktree `.claude/worktrees/agent-abe88f2eda1c6d085` at `3aca59434`: follow control, page keys, Esc/Enter back to
latest, footer hints (`src/pages/session/transcript/{follow,mod,strings}.rs`, `widgets/thread_ui.rs` `Follow`, `tests/pages/transcript_follow.rs`).
This brief is everything after that, plus the pager and backtrack. It replaces `pager.md`.

## 0. First step for the porter
- Start from tier 1: rebase `3aca59434` (or its landing) onto the integration tip `b801a3c8a` or later. Tier 1 and this chunk are one area (§5).
- Probe: `KIT_PROBE_PENDING=1 ~/.cache/rtwk/acargo nextest run --profile oracle -E 'binary(oracle) & test(/^(transcript_view_[1-5]|pager|composer_1)::/)'`,
  then `tests/oracle/tools/summary.py --list stale`. The tier-1 probe found 195 red, 21 stale pending, 52 n/a and 4 pending.
- Stale candidates (tier 1 covers them; un-pend the ones the probe passes):
  - tv1: `default_page_up_still_scrolls_the_transcript`, `empty_enter_returns_to_latest_with_contextual_hints`,
    `escape_returns_to_latest_without_changing_the_composer_draft`, `owned_transcript_reserves_a_row_above_the_composer`,
    `slash_picker_overlays_history_without_moving_the_transcript_or_composer`, `submitting_a_draft_from_history_queues_once_and_follows_immediately`,
    `visible_updates_and_empty_tail_cells_do_not_report_activity`;
  - tv2: `passive_footer_keeps_jump_actions_in_one_row`, `empty_viewport_does_not_show_back_to_bottom`,
    `enlarging_paused_view_hides_back_to_bottom_when_tail_fits`, `decimal_mouse_scroll_speeds_preserve_whole_rows`,
    `mouse_scroll_speed_scales_rows_and_accumulates_fractional_movement`;
  - tv5: `offsets_can_reach_rows_after_u16_limit`, `single_row_scrolling_crosses_an_entry_separator_in_both_directions`;
  - also `tests/rtwk_g9.rs`: it is green; keep it as the ADR-029 guard.
- `tests/oracle/pager.rs` has 23 `kit::missing` stubs with no bodies, all owned by ChatLive. Write the bodies from the oracle first (step 1), then set their owner to this chunk.
- tm housekeeping is the orchestrator's: tasks `pager` and `backtrack` carry stale areas (`src/screens/pager`, `tests/pager.rs`). Set them to the §5 list, or fold both into this job.

## 1. Scope (oracle)
- **Owned transcript view.**
  - Root: `transcript_view.rs` (803).
  - `transcript_view/`: selection 836, text 782 (517 highlight), text_tabs 303, text_logical 103, layout 439, search 958,
    search_presentation 119, search_footer 106, select_mode 507 + select_mode/motion 440, copy_mode 400, disclosure 490,
    turn_details 407, activity 33, prompt_header 76, footer 312, turn_tip 61, bookmark 46, mutations 256, snapshot 164, composer_gap 158.
  - Event routing: `app/owned_transcript.rs:811-1200`. The Esc order is at :974-1060:
    select_transcript → read-only Esc → shortcut overlay → vim insert → backtrack preview → backtrack prime/preview → close detailed → jump to latest.
- **Backtrack.**
  - `app_backtrack.rs` (1247): open/close :193-270, prime :270, preview :280-365, confirm :500-560,
    `backtrack_revert_before_turn_id` :572-685, unit tests :807-1238.
  - `app_backtrack/{legacy_input 250, browsing 96, prompt_navigation 71}`.
  - The revert flow is `app/event_dispatch.rs:820-960`:
    1. thread_read + hydrate;
    2. compute before_turn_id;
    3. `revert_thread` (`app_server_session/history.rs:169-180`, `ClientRequest::ThreadRevert{ThreadRevertParams{thread_id, before_turn_id}}`);
    4. wait for `ThreadReverted` (10 s timeout);
    5. hydrate retained turns through `thread/turns/list` (the response's `turns` is always empty);
    6. restore the prompt to the composer and truncate.
  - Messages: "Failed to edit the selected prompt: {err:#}", "No previous message to edit.",
    "Editing previous prompts is unavailable in side conversations.".
- **Pagers.**
  - `pager_overlay.rs` (605): `PagerView` header "/ " repeated + "/ TITLE" dim; `~` filler; bottom bar "─" + " N% ";
    StaticOverlay `HINTS_HEIGHT` 3, hint rows "to scroll / to page / to jump" + "close" + optional `PagerAction` (Enter "Edit configuration").
  - `pager_overlay/transcript.rs` (601): "/ T R A N S C R I P T" + hints. This is the TranscriptOverlay; it is used inline and by the resume picker (`resume_picker.rs:1464`).
  - StaticOverlay users:
    - /diff: `event_dispatch.rs:1547-1566`, "D I F F"; "No changes detected." italic; lines through `ansi_escape_line`.
      `get_git_diff.rs:51` runs git through `workspace_command.rs:131` = app-server `command/exec`. Messages: "`/diff` — _not inside a git repository_" and "Failed to compute diff: {e}".
    - Full-screen approvals: "P A T C H" / exec (:3685).
    - "U S E R  V E R I F I C A T I O N" (:3675).
    - Misalignment "What we detected" (`misalignment_policy.rs:43`).
    - Provider details + PagerAction (`app/provider.rs:179`).
- **Copy.**
  - /copy: `slash_dispatch.rs:490`. In owned mode without status targets → `SelectTranscriptCopy{guard}` → `copy_mode.rs` (this chunk).
    Otherwise the picker `chatwidget/copy_picker.rs` (chat-commands).
  - `markdown_copy.rs` 681 + `markdown_copy/table.rs` 242.
  - `clipboard_copy` (native/WSL/PRIMARY).
- **What the user sees.** A full-screen transcript the user can:
  - drag or keyboard-select and copy (as Markdown source where it is Markdown);
  - search with F3 or "/";
  - switch to detailed with Ctrl+T;
  - jump between prompts with Esc Esc and edit one with Enter, which reverts the thread;
  - see a sticky prompt header and fold or unfold tool/turn details.
  /diff and long outputs open a titled static pager with `~` rows and a "N%" bar.

## 2. Tests (in-chunk: 235 screen tests + ~41 copy unit tests + 8 core backtrack tests; pass now 0)
State is from the `04ff6c716` junit and the shard files. Ownership is from a name/body classification (scripts `/tmp/rtr/classify.py`).

| shard | total | pending | n/a | in chunk | other chunks |
|---|---|---|---|---|---|
| transcript_view_1 | 51 | 50 | 1 | 40 | 11: chat-live 5 (plan_menu x2, external_writer, provider_error, inline_confirmation); composer 5 (thumbnails, fullscreen composer x3, raw paste); pets/G4 1 |
| transcript_view_2 | 52 | 45 | 7 | 45 | – |
| transcript_view_3 | 50 | 16 | 34 | 16 | – |
| transcript_view_4 | 60 | 59 | 1 | 59 | – |
| transcript_view_5 | 59 | 50 | 9 | 50 | – |
| pager | 23 | 23 (missing) | – | 23 | – |
| composer_1.rs:89 | 3 | 3 | – | 3 backtrack footer snaps (functions only, composer owns the file) | – |

The 209 transcript_view tests in this chunk, by group:

| group | count | note |
|---|---|---|
| selection + copy | 49 | |
| find | 29 | |
| follow/footer | 16 | tier 1; stale candidates |
| select mode | 14 | |
| layout/text | 13 | wrap, tabs, resize anchor, prompt background |
| copy mode | 8 | |
| prompt header | 6 | |
| backtrack | 5 | plus browsing ones inside find/history/keymap |
| disclosure | 3 | |
| needs history pages | 26 | the kit helper, §3 |
| needs cells | 34 | recap, reasoning, plan, mermaid, table, exec, patch: wait for cells-markdown, then here |
| needs keymap seed | 6 | |

- The 52 n/a: tv2 7 + tv3 34 are "copied payload, nothing drawn" (markdown_copy, markdown_element_copy).
  Port them as unit tests over the copy module (step 4), the way status ported its n/a helpers; the other 11 stay n/a.
- pager 23 splits as follows:
  - Static: 5 (`static_overlay_snapshot_basic`, `static_overlay_wraps_long_lines` snaps; `pager_view_content_height_counts_renderables` state → unit;
    `scrolled_static_renderables_match_clipped_buffer_slices`, `fallback_handles_offsets_near_maximum_height`).
  - TranscriptOverlay: 18 (`pager_overlay/transcript_tests.rs`: select/find/focus, live tail, history prepend, page bindings, web links).
- Users of the static pager in other chunks' shards, enabled once the sheet lands: `approvals_2.rs:864`, `composer_2.rs:505`,
  `status.rs:393` (`transcript_overlay_remeasures_status_after_rate_limit_refresh`), and the resume ctrl+t preview.
- Core: 8 `app_backtrack.rs` unit tests → `core/tests/backtrack.rs`.
- Kit limits that keep tests pending until `tests/kit` grows them: no leader key (`ctrl-x [` select), no right-click, no shift-click,
  no triple click, no key release, no X11 PRIMARY. Each is a kit need (§5), not a gap.

## 3. What exists in ours, and the missing data
- **Transcript pane.** `src/pages/session/transcript_pane.rs` (349): `ListView::builder` over rows (exploration grouping, detailed-only items),
  a `SelectionArea` around a `NotificationListener` for history load, reverse when live and headerless.
  Tier 1 adds `Follow` (ListController + unseen notifier), Page keys, Esc/Enter to latest and footer hints.
- **Session page.** `session_page.rs` (802):
  - `ChatState.detailed_transcript` + `esc_primed` (:68-72);
  - `Action::OpenTranscript` toggles detailed (:338);
  - the Esc logic :358-387 calls `open(cx, Target::Backtrack)`;
  - `backtrack_hint` footer :560 (l10n `strings.rs:114` "Esc again to edit the previous message").
- **open.rs.** `open.rs:63-77`: `Target::Pager` is "not yet"; `Target::Backtrack` pushes the stub page `src/pages/session/backtrack/{mod,strings}.rs` (debug `entry!(backtrack, …)` at `src/app/debug/mod.rs:87`).
  Both are wrong shapes: they are modes of the session transcript, not pages.
- **Core stubs.** `core/src/screens/transcript.rs` and `backtrack.rs` (98 each) are empty stubs (Model/Ev/Intent/Command, NoSeeds).
- **Thread model.**
  - `core/src/threads.rs:78` `Item`.
  - `ThreadAsk::Turns`/`History{cursor}` → `thread/turns/list` (:1058-1070).
  - `can_load_history` :851, `read_only` :887.
  - ThreadReverted is decoded (`protocol/targets.rs:27`, `threads/previews.rs:109`, agents notifications); no thread state reacts to it.
- Other pieces already in place:
  - The resume page's ctrl+t is `NotYet` (`src/pages/resume/mod.rs:634`).
  - The copy picker fixture exists (`src/sheets/select/fixtures/copy_picker.jsonl`, chat-commands).
  - Markdown renders through `src/widgets/markdown.rs` on `rtwk_rich::markdown` Blocks, which have no source map.
- **Missing data, as exact requests:**
  1. Prompt restore needs what `core/src/threads/user_prompt.rs` `UserPrompt{text, elements: Vec<Range>, images: Vec<usize>}` drops.
     Keep `local_images: Vec<(PathBuf, Range)>` from `UserInput::LocalImage{path}`, `remote_images: Vec<String>` from `UserInput::Image{url}`,
     and the element placeholders (`TextElement` placeholder text) next to the ranges. The oracle restores all three (`app_backtrack.rs:500-560`).
  2. Revert: a ThreadFx `Command::Revert{thread, before_turn_id}` → `ClientRequest::ThreadRevert{ThreadRevertParams{thread_id, before_turn_id}}`.
     On `ThreadRevertResponse` then `ServerNotification::ThreadReverted{thread_id}` the thread re-hydrates via the existing `ThreadAsk::Turns`, and `apply` truncates.
     A 10 s timeout is an Ev with a ticket. ADR-003: only ThreadFx sends it; the screen sends `Intent::EditPrompt{thread, nth}`.
  3. /diff: a host effect `Command::GitDiff{cwd, ticket}` → `ClientRequest::CommandExec{CommandExecParams{command: ["git", …], cwd, …}}` × the
     `get_git_diff.rs` sequence (rev-parse, fsmonitor probe, diff config, tracked diff, untracked `--no-index`).
     It returns `Ev::GitDiff{ticket, Result<(inside_repo, String), String>}`. Port the git-command builder of `get_git_diff.rs` into core as pure functions (no IO), and the runner into services.
  4. History pages in tests: `src/services/backend/fake.rs:249` `FakeHandle::respond` can answer `thread/turns/list`.
     A kit helper `kit::history_pages(&[page…])` is needed; the fixture `script.rs` has no reply step.

## 4. Flutter → rtwk, per piece
- **Mode controller.** Flutter: a `ValueNotifier<TranscriptMode>` + `Shortcuts`/`Actions` per mode.
  Ours: one `TranscriptModeController` in `widgets/thread_ui.rs` next to `Follow`, holding
  `Following | Reading | Detailed | Browsing{nth} | Find{query} | Select{marks} | Copy{target}`.
  It replaces `detailed_transcript`/`esc_primed`, and the Esc order of `owned_transcript.rs:974-1060` becomes the controller's one `on_escape`.
  The thread state stays in `CodexApp`; the controller holds only view state (offsets, marks, query). No ADR-003 copy.
- **Selection + copy (ADR-028; superseded at the pin c953f062 by ADR-029, parity/repin-5.md, gap row G-new-sel-lines).** The list below was written for `ItemTexts` / `ListView::selection_text`; on the pin an item is one line of text (`ListView::item_text`, `ItemText`), the item draws its part through `ListView::selection_of`, there are no keys (a selection is kept across a change by `SelectionAreaController::start`/`end` and `select_between`), and a cell of many wrapped rows is not an item of it. `ItemTexts` is still in the tree until rtwk's `sel-028-retire`; do not build on it.
  - `TranscriptTexts: ItemTexts` over the rows the pane builds, with `key(i)` = the item id; `ListView::selection_text(Rc<dyn ItemTexts>)`
    (`rtwk-scroll/src/list_view.rs:503`) inside one `SelectionArea` (`rtwk-widgets/src/gestures/selection.rs:460`).
  - `SelectionArea` gives `copy_on_release`, `on_copy`, `copy_transform` and `trim_trailing_blanks`. The `SelectionAreaController` (:271) gives
    `has_selection`/`clear`/listeners for the selection footer and the right-aligned copy feedback (`tv1:copy_feedback_is_right_aligned…`).
  - The text of a mounted item must equal its rendered `Text`s joined by "\n". `revision`/`changed_from`/`dropped` carry live cells and history prepends
    (the tv3 "page join … revision" tests).
  - Clipboard: `rtwk::editing::copy_to_clipboard` (`rtwk-editing/src/editing_keys.rs:283`, OSC 52) + REUSE `clipboard_copy` native/WSL/PRIMARY (G11 partly closed).
- **Markdown-aware copy.** `ItemTexts::copy_into(index, start, end, out)` is what `ItemIndex::read` calls for every copied range (`selection_text.rs:752`).
  The app's `copy_into` maps the rendered byte range back to source through REUSE `markdown_copy` (`SelectedLine`/`CopyLine`).
  - The bytes must equal the rendered length, or the read returns "" (:753). So markdown output goes through `copy_transform` on the whole payload instead:
    `copy_into` writes rendered text and records the item ranges; `copy_transform` rebuilds the Markdown from those ranges.
  - Cells must give each Markdown item its source + per-line `CopyLine` (rtwk-rich Blocks have no source map; G-new below).
- **Find (F3, "/").**
  - Flutter: a `TextField` + `TextEditingController` in the footer.
  - Ours: `find_in_items(texts, query, from, forward)` (wraps once; matches inside one item) → `ListController::ensure_index_visible(i)` (:157)
    → `SelectionAreaController::select(SelectionTarget::of(..), i, start, end)` (:329). The oracle highlights matches with the selection style (`text.rs:517`), so one highlight serves both.
  - The "no wrap" legacy keys and older-page continuation: when `find_in_items` reaches index 0 with `can_load_history`, send `ThreadAsk::History` through the app and resume after the page lands.
- **Select mode** (keyboard caret, `select_mode/motion.rs` pure → REUSE into core): character/word/line motion over `ItemTexts` offsets, shown with `controller.select`.
  Three things are missing (G-new):
  - vertical motion with a preferred display column;
  - a selection between a mark in item A and a caret in item B (`select` names one item);
  - shift-click extend.
- **Copy mode** (/copy in owned mode, the leader select). Flutter: a focus traversal over blocks.
  - Ours: the target list in core (user turn / response blocks); `ensure_index_visible` + a highlight `DecoratedBox` on the targeted rows; vim keys via `Shortcuts`.
  - The /copy picker outside owned mode stays chat-commands'.
- **Backtrack browsing.** The mode `Browsing{nth}` highlights the nth user prompt and jumps with `jump_to_index_rows(i, ctx)` (:145).
  Left/Right/Esc/Enter come from the keymap TRANSCRIPT_RESERVED set. Enter → `Intent::EditPrompt` (§3.2); the composer is filled from the restored `UserPrompt`.
- **Static pager.** Flutter: a full-screen route (`showModalBottomSheet(isScrollControlled)`-like sheet) =
  `Column[title row, Expanded(LayoutBuilder → SingleChildScrollView(lines) + "~" rows to fill), "─ N% ─" from ScrollController
  pixels/max_scroll_extent, hints]`.
  - Ours: `src/sheets/pager/` (`StaticPager{title, lines, action: Option<PagerAction>}`) pushed with `overlay::show_dialog` full size.
  - /diff lines through `rtwk::widgets::ansi::parse_ansi` (`rtwk-widgets/src/ansi/stream.rs:29`; this resolves "ANSI parser").
  - The `~` filler is a `LayoutBuilder` (G23: not a gap; this resolves "~ filler widget").
- **TranscriptSheet** (TranscriptOverlay): the same sheet with the transcript `ListView` + `selection_text`, title "T R A N S C R I P T" and the pager keymap.
  It serves the resume preview now and the inline Ctrl+T later (G1).
- **Sticky prompt header.** Flutter: `SliverPersistentHeader(pinned)` / an `ItemPositionsListener`-driven `Stack` overlay.
  rtwk has `Stack`/`Positioned` (`layout/stack.rs:36/:107`) but no visible item range (G-new). Pending on it; no hand layout (R15).
- **Disclosure, turn details, activity groups.** Flutter: `ExpansionTile` + `ExpansionController`. Ours: `rtwk-controls/src/expansion.rs:47` per group,
  with the expanded set in the mode controller keyed by item id; find expands the match only (`tv4:find_expands_only_the_match…`, needs cells).
- **Scrollbar**: `rtwk-scroll/src/scrollbar.rs:145` where the oracle draws one.
- **Pager keymap context** (resolved): `Context::Pager` ActionIds exist in the keymap porter's `core/src/keymap/actions.rs:191-201`
  (`PAGER_SCROLL_*`, `PAGER_PAGE_*`, `PAGER_HALF_PAGE_*`, `PAGER_JUMP_TOP/BOTTOM`, `PAGER_CLOSE`, `PAGER_CLOSE_TRANSCRIPT`, `PAGER_FIND`)
  and `GLOBAL_{OPEN,FIND,SELECT}_TRANSCRIPT` :10-12 (worktree agent-a6569cfa1809bdfef, uncommitted).
- **Scroll to index** (resolved): `ListController::jump_to_index` :134, `jump_to_index_rows` :145, `ensure_index_visible` :157. Shrink-wrap exists (:609), so G53 looks closed.
- Gap rows:
  - G-new: ListView visible item range. Flutter has `ItemPositionsListener`/pinned sliver header; rtwk tried `ListController` + `ScrollController`, which expose only pixels.
    There is no first/last visible index, so the sticky prompt header, the find start "from the visible entry" and the resize reading anchor cannot be known.
  - G-new: keyboard selection. Flutter has `SelectionArea` keyboard extension (`GranularlyExtendSelectionEvent`, preferred column) and shift-click; rtwk tried
    `SelectionAreaController::select`. It selects within one item only, with no caret, no vertical display-column motion and no shift-click extend.
  - G-new: Markdown copy source. Flutter has a custom `Selectable.getSelectedContent` / `SelectedContentRange`; rtwk tried `rtwk_rich::markdown` Blocks + `copy_transform`.
    Laid-out lines carry no source byte range or block path, and `CopyTransform = Fn(&str)` (`selection.rs:235`) gets no item ranges, so a selection cannot be mapped back to Markdown.
  - G9 closed (ADR-028); G23 not a gap; G21 not a gap; G1 open (inline Ctrl+T); G11 partly closed; G4 (pets column, images in the transcript).

## 5. Area and conflicts
- **Area** (one job; it carries tier 1's area and adds the rest):
  - `src/pages/session/transcript/` (new: `mode.rs`, `texts.rs`, `copy.rs`, `find.rs`, `select.rs`, `copy_mode.rs`, `browse.rs`, `header.rs`, `disclosure.rs`, `strings.rs`);
  - `transcript_pane.rs`, `transcript_activity.rs`, `transcript_header.rs`, `transcript_prompt.rs`, `widgets/thread_ui.rs`;
  - `src/sheets/pager/` (new);
  - `src/pages/session/backtrack/` (delete the page; keep the strings);
  - `core/src/screens/transcript.rs`(+dir), `core/src/screens/backtrack.rs`, `core/tests/{backtrack,transcript_copy}.rs`;
  - `tests/oracle/transcript_view_1..5.rs`, `tests/oracle/pager.rs`, `tests/pages/transcript*.rs`, `docs/rtwk-port/parity/transcript.md`, `docs/rtwk-port/rtwk-gaps.md` (3 rows).
- **Shared files, with the minimal change each needs:**
  - `session_page.rs` (chat-live): the Esc/Ctrl+T/F3 arms (:338, :358-387) forward to the controller; drop `detailed_transcript`/`esc_primed`. About 30 lines.
  - `open.rs`: `Target::Pager`/`Backtrack` set the mode instead of pushing a page.
  - `src/app/debug/mod.rs:87`: the backtrack entry points at the mode.
  - `core/src/threads/user_prompt.rs` + `threads.rs` (app/startup-rest): the §3.1 fields; the §3.2 Revert command in ThreadFx + the ThreadReverted apply arm.
  - `core/src/app.rs` (keymap porter claims it): `Intent::EditPrompt`, `Command::GitDiff`, `Ev::GitDiff`. Sequence after keymap lands.
  - `src/services/shell.rs`: run `CommandExec` for GitDiff.
  - `tests/kit/` (keymap porter claims `seed.rs`): `history_pages`, shift-click, right-click, triple-click, key release, leader.
  - `composer_1.rs:89` (3 functions).
  - `src/l10n/strings.rs:114`.
- **Needs from other chunks:**
  - composer: `ComposerDraft::restore(UserPrompt)` (text + elements + images; `widgets/composer_draft.rs` has none).
  - keymap: the Pager/Global bindings fed into `Shortcuts` + the keymap seed for 6 tests.
  - cells: a copy source per Markdown cell + the 34 cell-dependent tests.
  - resume: opens the TranscriptSheet for its ctrl+t.
  - approvals/misalignment/provider/user verification: open the StaticPager.
  - chat-commands: dispatches /copy to copy mode when owned and to the picker otherwise; /diff to `Command::GitDiff`.

## 6. The plan (one job, tests first; each step names the shard groups it turns on)
1. **Tests and probe.**
   - Rebase on tier 1 and probe (§0); un-pend the stale ones.
   - Write the 23 `pager.rs` bodies from `pager_overlay.rs:568+`, `pager_overlay/scrolling_tests.rs:18+` and `pager_overlay/transcript_tests.rs`.
   - Add `kit::history_pages` over `FakeHandle::respond`.
   - Port the 8 `app_backtrack.rs:807-1238` tests into `core/tests/backtrack.rs` and the ~41 copy-payload n/a tests into `core/tests/transcript_copy.rs`.
2. **StaticPager sheet** (`src/sheets/pager/`, oracle `pager_overlay.rs` 1-560): title, `~` LayoutBuilder rows, "N%" bar, hints, PagerAction, Pager keymap.
   - Then /diff end to end: `get_git_diff.rs` builder into core, `CommandExec` runner in services, the `D I F F` sheet with `parse_ansi`.
   - Turns on: pager static 5; enables `approvals_2.rs:864`, `composer_2.rs:505`.
3. **Mode controller** (`transcript/mode.rs`), Esc order from `owned_transcript.rs:974-1060`; Ctrl+T detailed through it.
   - session_page/open.rs minimal change; delete the backtrack page.
   - Turns on: the follow/footer remainder, layout/text 13 (`transcript_view/{layout,text,text_tabs,text_logical}.rs`).
4. **Selection + copy**:
   - `TranscriptTexts` + `selection_text` + feedback + selection footer + copy-on-select + link-on-release (`transcript_view/selection.rs`, `footer.rs`).
   - Markdown copy via `copy_transform` + REUSE `markdown_copy` into core.
   - Turns on: selection-copy 49 (minus kit-limited), copy unit tests, and history-page selection ones.
5. **Find** (`search.rs`, `search_presentation.rs`, `search_footer.rs`): TextField footer, `find_in_items`, `controller.select`, older-page continuation.
   - Turns on: find 29 + its history-page ones + pager find/select.
6. **Select mode** (`select_mode.rs`, `select_mode/motion.rs` → core) as far as `controller.select` reaches. Cross-item and vertical-column ones go `pending(G-new keyboard selection)`.
   - Turns on: select mode 14 (part).
7. **Copy mode** (`copy_mode.rs`), /copy owned branch + leader. Turns on: copy mode 8.
8. **Backtrack**:
   - core state machine (`app_backtrack.rs:193-685`, `browsing.rs`, `prompt_navigation.rs`, `legacy_input.rs`);
   - the Browsing mode;
   - `Intent::EditPrompt` → ThreadFx Revert → re-hydrate → `ComposerDraft::restore`;
   - the error messages.
   - Turns on: backtrack 5, composer_1 3 snaps, the keymap 6 after the seed.
9. **TranscriptSheet** (`pager_overlay/transcript.rs`) over the same pane. Turns on: pager transcript 18 (the inline-only ones `pending(G1)`); resume wires it.
10. **Disclosure, turn details, activity groups** (`disclosure.rs`, `turn_details.rs`, `activity.rs`, `turn_tip.rs`, `bookmark.rs`) with ExpansionController.
    - The sticky prompt header (`prompt_header.rs`) is `pending(G-new visible range)` unless the person decides otherwise.
    - Turns on: disclosure 3, then the 34 cell-dependent tests once cells-markdown lands; prompt header 6 stay pending.
- **Finish:** run the probe again and report the pass count per shard; update `docs/rtwk-port/parity/transcript.md` and add the 3 G-new rows to `rtwk-gaps.md`.

## 7. Gaps, risks and decisions for the person
- **D1, chrome.** The oracle's owned full-screen Ctrl+T and backtrack draw no pager chrome; only the static pager and the TranscriptOverlay
  (inline, resume preview) do. Recommendation: chrome only on those two sheets; inline Ctrl+T waits on G1.
- **D2, markdown copy route.** `copy_into` must return exactly the rendered byte count, so the Markdown source cannot come out of it.
  Recommendation: rendered text through `copy_into`, Markdown rebuilt in `copy_transform` + REUSE `markdown_copy`.
  `copy_transform` gets only `&str`, so `TranscriptTexts` records the `(index, a, b)` of each `copy_into` call in a `RefCell` the transform shares.
  Reset it per read; this is fragile because `read` also serves `text()`, so assert on the length.
  It needs the cells chunk to emit a per-line copy source until rtwk-rich exposes one (G-new Markdown copy source). The alternative, a core pulldown_cmark pass whose logical lines must match rtwk-rich's, is fragile.
- **D3, sticky header and resize anchor** need the visible item range (G-new). Recommendation: pending with the row, no hand layout (R15).
  The person files the rtwk row; it unblocks 6 + ~3 tests.
- **D4, select mode across items / vertical column / shift-click** (G-new keyboard selection). Recommendation: port what `controller.select` reaches, pend the rest with the row.
- **D5, one job or two.** Tier 1's task "Owned transcript behaviour" already claims this area; the tm `pager`/`backtrack` tasks overlap it.
  Recommendation: one porter carries steps 1-10 sequentially in tier 1's worktree after tier 1 lands. Fold `pager`/`backtrack` into it. Parallelism comes from the other chunks (cells, composer restore, keymap seed), which this job only consumes.
- **Risk.** `core/src/app.rs` and `tests/kit/seed.rs` are claimed by the keymap porter: steps 2 (GitDiff) and 8 (EditPrompt) land after it, or ask it for the 3 variants.
- **Risk.** Revert is a destructive server call. A screen test must drive it only through `FakeHandle::respond`; the 10 s timeout runs under the fake clock.
