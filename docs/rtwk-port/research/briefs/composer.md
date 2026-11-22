# Recon brief: composer (chat composer, Vim, Ctrl+R, history, @ mentions, paste, attachments, editor)

Read-only recon of 04ff6c716 (oracle junit `codex-rs/tui-rtwk/target/nextest/oracle/junit.xml`), branch
claude/tui-rtwk. Paths below: oracle = `codex-rs/tui/src`, ours = `codex-rs/tui-rtwk`, rtwk = `rtwk/crates`.
Prior parity pack: `docs/rtwk-port/parity/composer.md` (297 lines; its decisions stand, see §3).

## 0. First step for the porter
1. Wait for the keymap porter to land (worktree `agent-a6569cfa1809bdfef`, tm "Keymap: Codex keymap config").
   It owns `core/src/keymap/` (contexts Composer, Editor, VimNormal, VimOperator, VimSearch, VimTextObject;
   ~140 `VIM_*` action ids), `tests/kit/seed.rs` `.keymap(..)` (now `missing()`, seed.rs:111), `core/src/app.rs`,
   `host/src/{facts,in_process,startup}.rs`. Vim, kills and remap tests here all need it. Rebase on it first.
2. Probe the stale pendings once, before any code:
   `KIT_PROBE_PENDING=1 ~/.cache/rtwk/acargo nextest run --profile oracle -E 'binary(oracle) & test(/^(composer_[1-4]|composer_popups|textarea)::/)'`
   then `python3 -I tests/oracle/tools/summary.py --list stale`. Candidates, by reading the reasons:
   - composer_3 `bare_slash_command_can_be_recalled_after_recording_pending_history`,
     `popup_selected_slash_command_records_canonical_command_history` (popup and recall exist now);
   - composer_3 `left_navigates_only_from_an_empty_prompt` (Left on an empty prompt opens agents, session_page.rs);
   - composer_2 `tab_queues_slash_led_prompts_while_task_running_without_validation`,
     `ascii_burst_treats_enter_as_newline_even_when_parent_owned`; composer_4 `editor_actions_insert_shell_prefixes`;
   - composer_popups `history_search_*` (7): Ctrl+R exists (session_page.rs:516-548) though the reasons say "not
     built"; highlight and the footer line likely stay pending (G-new below);
   - composer_popups `scrolling_mentions_shifts_rendered_window_snapshot` (the popovers seed takes ten entries now);
   - textarea `kill_current_line_*` (2) once keymap `editor.kill_whole_line` lands.
   Un-pend every stale one in its own commit, then start §6.

## 1. Scope (oracle)
- bottom_pane/chat_composer.rs (12820): docs :1-60; submit and history record :1280-1300, :3105-3200; set_text
  :1574-1620; Ctrl+C clear :1814; file search reply :1974; key routing :2025-2210; popups keys file :2210, skill
  :2296, mentions_v2 :2378; insertion :2590, :2873, :2894; no-popup keys :3585-4017; sync_popups :4018-4113
  (StartFileSearch :957); task search :965; history-search render :4922-4935. Tests :5017-12820.
- chat_composer/: history_search.rs (1237; begin :127, keys :166, footer :396, highlight :427-510),
  history_search_draft.rs (74), vim_history.rs (219, whole-draft undo 64 steps / 1 MiB), vim_register.rs (165),
  vim_search.rs, attachment_state.rs (224), completion_target.rs (358), paste_input.rs (313), popup_state.rs (135),
  draft_state.rs (60), reconnect.rs (164), inline_input.rs (211), mouse.rs (168), slash_input.rs (778),
  agents_navigation.rs.
- chat_composer_history.rs (1628): persistent + local in one offset space, on_entry_response :484, search :581,
  search_batch.rs.
- textarea.rs (4797): vim enable :323, mode label :448-470, keymap input :674, vim :798-1300 (insert :814, normal
  :843, operator :977, text object :1021, motion :1055), kill/yank :1303-1473, wrap :2162, render :2302-2368
  (elements cyan, highlights, selection, OSC 8). textarea/: vim.rs (336), vim_commands.rs (745), vim_search.rs
  (251), register.rs (73), wrapping.rs (225), hyperlinks.rs (98), mouse.rs (189), editing.rs (85), single_line.rs.
- Popups: mentions_v2/ (popup.rs 172, render.rs 396, search_catalog.rs 312, filter.rs, candidate.rs),
  skill_popup.rs (495), file_search_popup.rs (301), command_popup.rs (711, the composer's slash popup only),
  mention_usage.rs (191), paste_burst.rs (610).
- Outside bottom_pane: file_search.rs (133), external_editor.rs (316), clipboard_paste.rs (574: image :127/:238,
  path :257), mention_codec.rs (670), task_mentions.rs (371), attachment_files.rs (54); app/thread_routing.rs
  :668-740 (message history IO); chatwidget.rs:1814 (`/vim` toggle and "Vim mode enabled." / "Vim mode
  disabled."); app/input.rs:332 (Ctrl+G editor), :790 (`global.toggle_vim_mode`); constructor.rs:319
  (`tui.vim_mode_default`).
User sees: the `›` prompt with a wrapping draft, cyan placeholders (`[Pasted Content 1200 chars]`, `[Image #1]`,
`@file`, `$skill`), the Mentions popup with tabs above it, Ctrl+R `reverse-i-search:` on the footer row, Vim
(`Vim: Normal` magenta / `Vim: Replace` cyan on the footer right), Up/Down across sessions, Ctrl+G opens $EDITOR.

## 2. Tests (junit of 04ff6c716; pass / n/a / pending; 0 red, 0 todo)
| shard | pass | n/a | pend | total | pending groups (count) |
|---|---|---|---|---|---|
| composer_1 | 8 | 1 | 41 | 50 | mentions/skills/plugins 20, vim 10, chat-status footer 9, other 2 |
| composer_2 | 28 | 4 | 18 | 50 | mentions 5, keymap 3, footer 3, other 3, images 2, alone frame 1, reconnect 1 |
| composer_3 | 31 | 2 | 23 | 56 | mentions 5, images 4, other 4, vim 3, keymap 2, footer 2, reconnect 2, agents 1 |
| composer_4 | 12 | 2 | 59 | 73 | vim 37, reconnect/offline/handoff 7, images 4, keymap 4, footer 3, other/mouse/mentions/alone 4 |
| composer_popups | 12 | 0 | 31 | 43 | history search 7, popup G68 5, mentions 4, vim 4, other 4, alone frame 3, footer 2, images 1, keymap 1 |
| textarea | 11 | 2 | 46 | 59 | wrap/caret G65 26, OSC 8 links 11, vim 5, other 3, keymap 1 |
| total | 102 | 11 | 218 | 331 | |
Other chunks' tests (22, name and move on):
- chat-status (9, composer_1): `footer_hint_row_is_separated`, `footer_flash` x2, `footer_mode_snapshots`,
  `footer_collapse_snapshots`, `deleting_prefix_keeps_bang_editable_snapshot`, `question_mark_only_toggles_on_first_char`,
  `shift_question_mark_toggles_shortcut_overlay_when_empty`, `status_line_hyperlink_marks_pr_number_cells`.
- transcript (4): `kill_buffer_persists_after_slash_command_dispatch` (/diff pager); composer_popups
  `fallback_completions_keep_the_draft_cursor_and_footer_anchored`, `mention_scopes_overlay_history_without_moving_composer_or_footer`,
  `mention_menu_reserves_height_above_composer` (owned transcript plus status line).
- chat-commands (3): `slash_init_dispatches_command`; draft handoff x2 (startup draft into the session).
- cells-markdown (1): `blockquote_paste_large_expands_quoted_text_on_submit` (user-message cell).
- chat-live (1): the `unavailable_side_conversation` flag seed.
- (decision D4) embedded plain-text composer (4): `embedded_composer_shows_validation_error`,
  `blockquote_paste_embedded` x2, `plain_text_composer_keeps_command_prefixes_literal`.
In-chunk: 309 (incl. D4's 4) = 102 pass + 11 n/a + 196 pending. In-chunk tests that also need the chat-status
footer right side (`100% context left`, `Vim: ...` label): `image_placeholder_snapshots`, `blockquote_paste_rendered`,
`paste_burst_tabs_preserve_multiline_draft`, `plain_and_shift_spaces_insert_normally`,
`skill_completion_attaches_typed_punctuation_snapshot`, every Vim label snapshot; they land when chat-status does.

## 3. What exists in ours
- `src/pages/session/composer/mod.rs` (522): Composer widget over `Rc<ThreadUi>`; debug entry `composer/textarea`
  :49-59; `ComposerMsg` :182-208 (Changed, Submitted, Pasted, Picked, Completed, Refresh, Redraw, Highlight, Click,
  Dismiss, ClearForCtrlC, Shell, BurstDue); popup Popover + OptionList :246-285 (G68 look); burst :290; build
  :436-521 (TextArea, max_lines 6, on_paste, `EditingKeys` with `KeyData{draft, history, tell, burst, suggest}`,
  prompt `›` or bold `!`).
- `composer/editing.rs` (379): take_key :42, typed_char :69, edit_key :96-243: shell exit, atomic Backspace/
  Delete/Left/Right over elements, Up/Down history, Ctrl+K/U/W, Alt/Ctrl+Backspace, Alt+D, kills, Ctrl+C, Ctrl+Y.
- `composer/suggest.rs` (235, route (b) Suggest model), `burst.rs` (316), `footer.rs` (61, FooterLine), `strings.rs` (72).
- `src/widgets/composer_draft.rs` (731): elements, kill ring, history walk, shell mode, paste, attach_image :272,
  relabel_images :319, expanded :412, submission :420, snap_caret :522, kill/yank :540/:557, history :605/:620,
  cleared :667. Placeholders delete atomically through the controller (the person's decision, parity pack).
- `src/pages/session/popup.rs` (85: suggestions :20, commands :42, mention :65), `search_key.rs` (57),
  `popovers/` (104), session_page.rs Ctrl+R render :516-548, `ChatMsg::Search` :390-409, `Action::SearchPrevious` :339.
- core: `screens/popovers.rs` (421: app-scoped `Model{files, mentions, history, search}`, `HistorySearch` new/
  preview/set_query/step, Intents Record/Search*; seeds `{"files"}`, `{"mentions"}`, `{"history"}`);
  `screens/composer.rs` (286: paste thresholds, sanitize, blockquote, trigger :215) and `composer/paste_burst.rs`
  (489, the carry precedent for pure logic in core); `commands/dispatch.rs:396` `Vim => SessionIntent::Vim` falls
  through `app/session.rs:67-140` to "not available"; `:367` `Mention => Insert("@")`.
Missing: Vim entirely; persistent history; Ctrl+R match highlight, footer line and batches; real file search;
mentions catalog (skills, apps, plugins, tasks), mention tabs; Ctrl+V image from clipboard; remote image URLs;
Ctrl+G editor; reconnect/offline composer; OSC 8 links in the draft; composer-alone frame.
Data and where it enters:
- File search: app-server `fuzzyFileSearch/sessionStart|sessionUpdate|sessionStop` (common.rs:1608-1623, params
  :1971-2032) + notifications `fuzzyFileSearch/sessionUpdated` / `sessionCompleted` (:2145-2146); app-level
  (not thread state): `core/src/protocol` request + `AppEv`, results into `popovers.files`. Oracle uses in-process
  `codex-file-search` (file_search.rs) instead: D1.
- Mentions: `skills/list` (:950), `app/list` (:1041), `plugin/list` (:980) app-level into `popovers.mentions`;
  task mentions `thread/search` (:883) / `thread/list`: app-level list requests, not ThreadFx (no thread resumed).
- Persistent history: no protocol request; host IO via `codex-message-history` (`history_metadata`, `append_entry`,
  `lookup`, `lookup_batch`; message-history/src/lib.rs:41-300) as core `Command` variants run by host (pattern:
  `Command::Directories`/`SaveGrouping`, core/src/app.rs:227+), replies as `AppEv` into popovers.
- Clipboard image: host `arboard` (oracle clipboard_paste.rs, REUSE) as a host effect; host/Cargo.toml has neither.

## 4. Flutter -> rtwk
| piece | Flutter | rtwk (path) |
|---|---|---|
| draft | TextField + TextEditingController | `TextArea` rtwk-editing/src/text_area.rs:175-328, `TextEditingController` text_editing.rs:418 (kept) |
| key routing, modes | Focus.onKeyEvent / Shortcuts+Actions | `EditingKeys` rtwk-editing/src/key_hook.rs:151 ("Vim's modes are the application's", :123) |
| keymap chords, `g g`, `d i w` | Shortcuts with LogicalKeySet sequences | `KeyBindings` rtwk-framework/src/focus/shortcuts.rs:1257 (sequences, leader :382), via the keymap porter |
| Vim engine | a pure controller class | own pure module in core (paste_burst precedent); no widget |
| styled placeholders, match highlight, links | TextEditingController.buildTextSpan | none: G-new-1 |
| Vim whole-draft undo | UndoHistory over TextEditingValue | own in ComposerDraft (controller undo is text-only, text_editing.rs:549/:560) |
| mentions popup | OverlayPortal + ListView + TabBar | composed rows (Row/SizedBox, `›` Text) above the composer per G68 route 2; tabs ours widgets/tab_strip.rs or `TabBar` rtwk-controls/src/tabs.rs:237; cursor ours widgets/options.rs |
| history search | SearchAnchor | ours HistorySearch (core) + footer Text; highlight G-new-1 |
| external editor | Process.run($EDITOR) | `platform::run_on_terminal` rtwk-framework/src/platform/io.rs:222 runs it but nothing suspends the app from a handler: G67 |
| clipboard image | Clipboard / super_clipboard | none in rtwk for images: G11 (host arboard) |
| wrap and caret | TextPainter | G65 (26 textarea tests) |
| Ctrl+J newline | Shortcuts | G66 |

## 5. Area and conflicts
Claim (tm area), all under `codex-rs/tui-rtwk/`:
`src/pages/session/composer/{mod,editing,suggest,burst}.rs`, `src/pages/session/composer/vim*.rs` (new),
`src/pages/session/composer/fixtures/`, `src/widgets/composer_draft.rs`, `src/pages/session/{popup,search_key}.rs`,
`src/pages/session/popovers/`, `core/src/screens/composer.rs`, `core/src/screens/composer/` (new `vim/`,
`history.rs`), `core/src/screens/popovers.rs`, `core/tests/{composer_shell,popovers}.rs`, `core/tests/composer_vim.rs`
(new), `tests/oracle/{composer_1,composer_2,composer_3,composer_4,composer_popups,textarea}.rs`,
`tests/oracle/fixtures/composer_popups/`, `tests/pages/{composer,composer_burst,composer_draft,composer_images,
composer_mouse,composer_startup,composer_suggest,popovers}.rs`, `docs/rtwk-port/parity/composer.md`.
Shared, minimal change (orchestrator sequences):
- `src/pages/session/composer/{footer,strings}.rs` (transcript claim; chat-status brief): Vim mode label and
  `reverse-i-search:` footer Line, one variant each in FooterLine.
- `src/pages/session/session_page.rs` (chat-live): move the Ctrl+R preview into the composer slot; nothing else.
- `src/pages/session/keys.rs`, `tests/pages.rs`, `tests/screens.rs` (transcript): register new page tests only.
- `core/src/app/session.rs` + `core/src/commands/dispatch.rs`: handle `SessionIntent::Vim` (toggle UiApp vim flag,
  info cell "Vim mode enabled."/"disabled.").
- `core/src/app.rs`, `host/src/` (keymap): Command variants HistoryMetadata/HistoryAppend/HistoryLookup/
  HistoryBatch, FileSearch start/update/stop, ClipboardImage; host runs them.
- `host/Cargo.toml`: `codex-message-history`, `arboard` (own commit; `codex-file-search` only if D1 = in-process).
- `tests/kit/mod.rs`: SUPER/kitty key modifiers (2 tests), terminal default colours (light palette), right click.
Needs (the porter does not build them): the resolved `Keymap` provided to the view and `.keymap(..)` seed (keymap
porter); the footer right side `100% context left` + mode indicator (chat-status); owned transcript above the popup
(transcript); a disconnected/reconnecting seed and a parent-owned thread seed (chat-live).

## 6. Plan (one job; each step tests first: un-pend the named group, see it red, make it green)
1. Probe and stale un-pend (§0). Files: shards only.
2. Composer-alone debug entry like `composer/textarea` (mod.rs:49-59) for the oracle's composer-only frames
   (alone frame 5). Oracle chat_composer.rs:5050 test harness.
3. Keymap wiring: read `Keymap` in `editing.rs` instead of fixed keys; Composer/Editor contexts (submit, queue,
   newline, moves, kills, yank, history search). Turns on keymap 11 + textarea kill_current_line. Oracle
   textarea.rs:674-797, chat_composer.rs:2025-2064.
4. Vim engine, pure in `core/src/screens/composer/vim/` (mode, pending operator, motions, text objects, registers,
   dot repeat, search, counts) over a text+caret value; unit tests carry vim_commands.rs (745) and textarea vim
   state tests. Oracle textarea/vim.rs, vim_commands.rs, vim_search.rs, register.rs, textarea.rs:798-1300.
5. Vim in the view: `composer/vim.rs` applies engine edits through the controller and `ComposerDraft` (elements
   stay atomic); VimHistory whole-draft undo in ComposerDraft (vim_history.rs); mode in UiApp.ui; `/vim`, config
   `tui.vim_mode_default`, `global.toggle_vim_mode`; label and `/` search query on the footer row. Turns on vim 59.
   Oracle chat_composer/vim_*.rs, chatwidget.rs:1814, textarea.rs:448-470.
6. Persistent history: core `composer/history.rs` (persistent + local offsets, cache, pending lookups) from
   chat_composer_history.rs; Command/AppEv via host `codex-message-history`; record on submit with text elements,
   images, mentions, pastes (HistoryEntry). Up/Down across sessions.
7. Ctrl+R: move HistorySearch into the composer, batches (search_batch.rs), footer `reverse-i-search: q  enter
   accept · esc cancel`, draft restore, case-insensitive match ranges (history_search.rs:127-510). Turns on history
   search 7 (highlight waits on G-new-1).
8. Mentions: popup route 2 (rows composed above the composer, `›` marker, name/description columns, right type tag,
   hint line, tabs All Results / Filesystem Only / Plugins), file search requests (D1), catalog from skills/apps/
   plugins/tasks, insertion and mention bindings (mention_codec.rs, mention_usage.rs). Turns on mentions 35, G68 5.
   Oracle mentions_v2/*, chat_composer.rs:2210-2440, :2590-2900, :4018-4113.
9. Images and attachments: Ctrl+V clipboard image (host), path paste (clipboard_paste.rs:257), remote URLs restore,
   attachment_state.rs relabel. Turns on images 11.
10. Reconnect/offline/disabled composer (reconnect.rs) and Ctrl+G editor (external_editor.rs; G67 stays pending
    unless D3). Turns on reconnect 11 minus seeds from chat-live.
11. Remaining "other" 16 and mouse 1 one by one; update parity pack; textarea G65/OSC 8 stay pending on gaps.

## 7. Gaps, risks, decisions
- G-new-1: TextArea paints the controller's text in one style; no per-range style or link (Flutter
  TextEditingController.buildTextSpan). Blocks cyan elements, plugin accent, Ctrl+R highlight, 11 OSC 8 tests.
- G65 keeps 26 textarea wrap/caret tests pending; G66 Ctrl+J; G67 Ctrl+G; G68 popup look; G11 clipboard image.
- Risk: keymap lands late; steps 3-5 wait on it (steps 2, 6-9 do not).
- Risk: popovers Model is app-scoped while drafts are per thread; Ctrl+R state must move with the composer.
- Risk: the kit lacks SUPER/kitty keys, default colours, right click (tests/kit/mod.rs).
- D1 file search: app-server `fuzzyFileSearch/session*` (works remote) vs oracle's in-process codex-file-search.
  Recommend app-server.
- D2 Vim engine: pure in core (testable without rtwk, paste_burst precedent) vs in the view. Recommend core.
- D3 Ctrl+G: keep pending on G67 vs a host-run editor that suspends the app (host owns `App::suspend`).
  Recommend the host effect if the host can call suspend; else keep G67.
- D4 embedded plain-text composer (question sheets, 4 tests): this chunk (a `Composer` plain variant) vs the
  approvals chunk. Recommend this chunk.
