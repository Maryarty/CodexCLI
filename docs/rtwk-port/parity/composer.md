# composer — parity

S10, the SessionPage composer. Oracle `bottom_pane/chat_composer.rs`, `textarea.rs`,
`chat_composer_history.rs`, `chat_composer/{paste_input,paste_tests,skill_completion_tests,
blockquote_paste_tests}.rs`. View `src/pages/session/composer/`:
- `mod.rs`: the `Composer` widget: the prompt (`›`, or shell mode's light-red `!`),
  then `TextArea` under `Autocomplete`, then `EditingKeys`. `take_submission` is
  what Enter and Tab send. `ComposerMsg::Picked` tells a pick of the popup.
- `editing.rs`: Codex's keys (atomic placeholders, kill ring, history at the ends,
  shell mode in/out, Ctrl+C clear, a typed punctuation mark taking a `$skill`
  completion's separator space).
- `src/widgets/composer_draft.rs` (moved down from `pages/session/composer/draft.rs`,
  composer-2; rules test `widgets_never_import_pages`): `ComposerDraft`, the per-thread companion of the `TextEditingController`
  in `ThreadUi::draft` (elements, kill ring, history walk, shell mode, paste, the
  separator rule of a completion).
- `footer.rs` (with `with_shell_mode`, the right-hand `Shell mode` label), `strings.rs`.

Core (no rtwk), `core/src/screens/composer.rs`: `large_paste`, `blockquote_paste`,
`sanitize_paste` (unit test in the file); `screens/popovers.rs` lists skills under `@`.

## chunk-composer (the whole composer as one job; in progress, steps 1-9 of 11 landed; 10 reconnect/offline first half (core + draft) landed, the view wiring next)
Branch `worktree-agent-a8b7d6f0f27a204d2` (rebased on `claude/tui-rtwk` 2026-11-22).

Done:
- Keymap wiring: Composer/Editor contexts (submit, queue, the 17 editor actions,
  `kill_whole_line`); every key of the composer goes through `crate::keymap`.
- Vim engine in core (`core/src/screens/composer/vim/{mod,engine,motions,history}.rs`):
  pure, a `VimText` host trait, `VimMode` (not `Mode`: R14 is name-based and the agents
  page has its own `Mode`), 19 state tests from `vim_commands_tests`, `History<S>` (64
  steps, 1 MiB) with 3 unit tests.
- Vim in the view: `src/pages/session/composer/vim.rs` (keymap Vim contexts into `Input`,
  chords from the `.keymap` view API, the `/` query in its own controller, Insert/Replace
  recording for `.`, `u`/Ctrl+R through the draft's history), `src/widgets/composer_draft/vim.rs`
  (`Modal` UI state, `Snapshot` of the whole draft, `VimHost` = `VimText` over the controller plus
  the draft's atomic elements and the kill ring as the register). A paste in Normal mode is
  its own undo step (the draft inserts a plain paste itself in Vim, the step has to close with it).
  11 page tests in `tests/pages/composer_vim.rs` (`global.toggle_vim_mode` on F9).
- Turned on: textarea 14 -> 18, composer_4 12 -> 16 (see counts).
- Step 8a, mentions in pure core (b1059275f, 3378b4719): `core/src/screens/composer/mentions/`
  (no rtwk, no IO, no prose; 64 unit tests, the oracle's carried where it has them):
  - `target.rs`: `Doc{text, cursor, elements}` and the element-aware token range
    (`current_prefixed_token_range[_with_dollar_predicate]`, `dollar_query_kind` arbitrating
    shell variables, `prefixed_token_range_is_editable`, `ends_plaintext_at_mention`).
  - `codec.rs`: `mention_codec` over `history::TextElement` (`[$name](path)`, `[@name](thread://id)`);
    `task.rs`: `TaskMention{thread_id: ThreadId, ..}`, `parse/format_task_link`; `usage.rs`:
    `MentionUsage` (50-pick window).
  - `candidate.rs` / `catalog.rs` / `filter.rs` / `search_mode.rs` / `popup.rs`: the unified `@`
    popup as values: `Candidate`, `Capability{Skills, McpServers(n), Apps(n)}` (the words are
    the view's table), the oracle's sort (Plugin < Skill < Task < File; files by score), tabs
    `SearchMode{Results, FilesystemOnly, Tools}`; highlight and scroll belong to view controllers.
  - `dollar.rs`: the legacy `$` popup (skills and apps; Exact/Prefix/Fuzzy tier, frequency).
  - `files.rs`: the file-search seam. `FileSearchAsk{Query, Stop}` values and
    `trait FileSearchBackend`; the default `AppServerSearch` speaks `fuzzyFileSearch/session*`
    (start once, update, stop; answers filtered by session id); `FileSearch` keeps old matches
    until the answer (oracle) and has the "loading..."/"no matches" state.
  - `session.rs`: `Session` per composer (`sync(doc, sources) -> Vec<FileSearchAsk>`,
    `dismiss` by query + token + occurrence, `pick -> Pick{range, action, usage}`,
    `completion_separator`, `can_switch_search_mode`).
  - `popovers.rs`: `Ask::FileSearch`, `Ask::RecordMention`; `Ev::Tasks`, `Ev::FileMatches`,
    `Ev::MentionUsage`; `Intent::SearchFiles`, `Intent::RecordMention`; seeds `tasks`,
    `file_matches`, `mention_usage`, mentions with `path`/`plugin_id`/`capabilities`.
  - `history.rs`: `HistoryEntry::persisted(text, at_mentions_enabled)` decodes mentions and task
    ranges; `set_at_mention_restore_enabled`.
  Step 8a counts: whole `--workspace` nextest 2092 passed, 17 skipped; fmt, clippy
  `--workspace --all-targets -D warnings`, `shards.py --check` (55 shards, 2560 tests, 0 stale): clean.
- Step 8b, the mentions view (route 2: rows composed above the composer, G68 not needed):
  - `src/widgets/composer_draft/mentions.rs`: `Mentions` (the draft's `Session` plus the
    controllers the popup shares with the key hook: a `SelectionController<usize>` for the
    highlighted row, a `ListController` for the scroll, a `ChangeNotifier` the popup listens to;
    `sync`/`close`/`dismiss`/`step`/`switch_search_mode`/`set_search_mode`/`pick`/`choose`) and
    the `ComposerDraft` side: `sync_mentions` (closes on a selection), `dismiss_mentions`,
    `mention_pick`, `apply_pick` (a path as text, an image file as the next `[Image #N]` through
    `images::pasted_image`, a skill/plugin/task as an atomic element with its binding; a space
    first after an element; one separator after; only the token is replaced),
    `history_browsing` (`browsing_history` of `sync_popups`). The history round-trip keeps the
    binding (`Element.binding`, `bind_mentions`: `[$name](path)` and `[@name](thread://id)`).
  - `src/pages/session/popovers/mention.rs`: `MentionPopup{ui}` (stateful; rebuilds on the
    `Mentions` notifier, never in `build`): the unified `@` popup (title, `Tabs` strip with taps,
    the query line, the `↑`/`↓` overflow rows, a `ListView` with per-row `GestureDetector`, the
    hint line; height <= 17) and the `$` popup (rows + 3, no arrows, no tabs). Columns by a
    measured width (`LayoutBuilder` + `segmenter_of`, a measure closure into the row widgets): the
    primary column (file name first, parent path dim after), a 6-cell type tag, the description
    column (70% cap, hidden under 24 cells). Strings in `popovers/strings.rs` (ADR-010).
  - `src/pages/session/composer/mentions.rs`: the key hook (Up/Ctrl+P, Down/Ctrl+N, plain
    Left/Right switch the `@` tab only while the caret is in the token, Esc dismisses this token,
    Tab picks, Enter picks; Enter with no row submits for `@` and is consumed for `$`), `sync`
    (none for a bare field, history search, the Vim `/` query or a walk of the history; shell mode
    does not close it: `popups_enabled` is all the oracle checks), `pick` (apply, then
    `Intent::RecordMention(path)` for a skill and `Intent::SpendTasks` for a task), the file
    searches it asks as `Intent::SearchFiles`. The slash popup stays down where `@`/`$` claims the
    caret (`allow_command_popup`). `ComposerMsg::{MentionPick, SyncMentions}`; a build that finds the
    popup up self-sends `SyncMentions` (the sources may have changed), which never rebuilds the
    composer, so there is no loop.
  - Retired: `Model::candidates`, `Sigil`, `Candidate`, `CandidateKind`, `Ev::Files`,
    `Model::files`, the `files` seed (the fixtures use `file_matches` now) and the `$`/`@`
    branches of `src/pages/session/popup.rs` (the slash popup only); two core tests of them.
  - Tests: `tests/pages/composer_mentions.rs` 13 (8 key/draft tests on the whole page: Tab, Down
    wrap, Ctrl+N/P, Esc until the token changes, Enter with no row on `$` and `@`, Left/Right on
    `@` vs `$`, history round-trip as an element; 5 on the popup widget mounted alone with a
    synced draft: parts, a click on a row of `@` and of `$`, a tap on a tab, nothing drawn
    closed; the Esc test was shown biting by making Esc return false), 4 unit tests of the
    section arithmetic. Five older page tests that read the rows of the popup on the page are
    `#[ignore = "pending(mount)"]` (`composer_mouse::a_click_on_a_popup_row_picks_it`,
    `composer_suggest::a_click_on_a_mention_row_*` (2), `popovers::skill_popup_category_tags`,
    `popovers::default_unified_mention_popup`).
  - NEED (session_page.rs, chunk-live's file; the claim guard refused the edit, no bypass):
    mount the popup above the composer. In the bottom pane `Column` of `SessionPage`, in the
    `else` of `if thread.read_only()`, after the `transcript::follow::gap(...)` child and before
    the keyed `BoxWidget(prompt_surface ...)` composer child, add
    `.child(crate::pages::session::popovers::mention::MentionPopup::new(w.ui.clone()))`
    (it draws nothing while no popup is up). Then un-`ignore` the five tests above, probe the
    oracle shard tests with `KIT_PROBE_PENDING=1` (`default_unified_mention_popup_snapshot`,
    `mention_*`, `long_mentions_*`, `live_picker_frame_snapshots`, ...) and un-pend those whose
    bodies pass; until then the popup's look is checked by the widget tests only.
  - Departures and risks: (1) N1: an ellipsis where the oracle cuts (rich rows and the hint):
    rtwk's `dots_end`/`Text` ellipsis against the oracle's own cut. (2) The selected row is
    `tokens.selected()` over the whole row (the picker's), the oracle snapshot shows
    BOLD|REVERSED on an unknown palette; cannot be compared before the mount. (3) Page Up/Down in
    `transcript/follow.rs` still ask `trigger(before).is_none()` (a word that starts with `$`/`@`)
    to know whether a popup is up; `ui.draft.mentions().is_open()` is the exact answer
    (chunk-transcript-2's file). (4) `tag_skill`, `tag_app`, `tag_plugin` of
    `src/pages/session/strings.rs` are dead now (the popup uses `popovers/strings.rs`).
    (5) A PostToolUse hook reported that one of my commands wrote
    `docs/rtwk-port/research/pending-clusters/clusters.json`: it did not (that file is untracked in
    the main checkout, not in this worktree).
- Step 9, images and attachments:
  - Remote image rows (`chat_composer/attachment_state.rs`): `src/widgets/composer_draft/remote.rs`
    holds the URLs, the selected row and the caret the row was selected at (`remote_up`/`remote_down`
    /`remote_remove`, `set_remote_images`/`take_remote_images`/`forget_remote_images`); local images are
    numbered after the remote ones (`attach_image`, `relabel_images`), a kept entry and a sent entry
    carry `remote_image_urls`, `recall` restores them, the Vim `Snapshot` holds them (undo restores
    a deleted row). The view (`composer/mod.rs`) draws a `Column` of `Padding(2)`+`Text` rows
    (`tokens.info()`, selected = reversed), a one-row `SizedBox`, then the draft row. Keys in
    `composer/editing.rs` `remote_key`: Up at caret 0 selects the last row, Up/Down move, Delete or
    Backspace remove one row as one Vim edit (`vim_direct`, the `.` repeat is forgotten:
    `Engine::forget_repeat`), any other key clears the selection; a caret that stands elsewhere than
    where the row was selected ends it (checked in the `Refresh` arm, not on every controller
    notification: the relabel after a delete would clear it otherwise).
  - Ctrl+V (and Alt+V, any `v` with CONTROL/ALT; a plain `v` is typed): a chat-level key before the
    bottom pane, a repeat within 250 ms is swallowed (`composer_draft/clipboard.rs`, `REPEAT_WINDOW`).
    The view only emits `Intent::PasteImage`; core `popovers`: `Ask::ClipboardImage` (the host reads the
    clipboard into a temp PNG), `Ev::ImagePasted(path)` (seed `{"image_pasted":"/path"}`) and
    `clipboard::Pasted{answers, latest}` (core/src/screens/composer/clipboard.rs). The composer that
    asked records `answers` and attaches the latest once it grows (`image_arrived` /
    `take_pasted_image`, a self-sent `ComposerMsg::ImageArrived` from `build`, never a mutation in
    build), so a composer that did not ask never attaches an answer. The attach is the oracle's
    `attach_image` under `begin_direct_vim_edit`/`finish_vim_edit`.
  - Tests: `tests/pages/composer_images.rs` 20 (6 new draft tests: numbering after remote rows,
    renumbering on set/take, Up/Delete walk, Down past the last, a moved caret ends the selection, a
    kept draft comes back with its rows; 7 on the whole page: rows `["  [Image #1]","  [Image #2]","",
    "› describe these"]` (rows 1-4 of `remote_image_rows`) and after `home up up delete`
    (`remote_image_rows_after_delete_first`), up-up-delete with a local image renumbered, Up off the
    start is the field's, a click ends the selection, Ctrl+V + `Ev::ImagePasted` attaches and the
    turn carries the file, an unasked answer is not attached, a second Ctrl+V within the window asks
    once), 6 key-level tests in `composer/editing.rs` `tests::images` (shown biting: the 250 ms
    window made 0), 2 core tests (`Pasted::after`, the `image_pasted` seed). The remote presets go
    through the blank `ThreadUi` of `UiApp.ui.thread_ui.get(None)` before the thread exists.
  - NEEDS (not mine): (1) host: run `Ask::ClipboardImage` (arboard, G11: temp PNG, answer
    `Ev::ImagePasted`, the error cell `Failed to paste image: {err}` on failure), host/src and the
    drain of `take_asks` in `core/src/app.rs`. (2) `UserMessage` has no `remote_images`: a submit
    drops the URLs (the oracle sends them as `UserInput::Image` items before the local ones); and
    nothing calls `set_remote_images` on a restored message yet (the restore wiring of a
    rejected/queued message). (3) kit: a seed for remote image URLs (`Screen::remote_images`), the
    reason of `remote_image_rows_snapshots`, `history_navigation_restores_remote_only_submissions`,
    `deleting_a_selected_remote_image_is_one_undoable_vim_edit` and
    `selected_text_navigation_does_not_recall_history_or_select_images`; the first also needs the
    oracle's own footer (pane-footer). (4) The oracle's cap of the rows at `min(count, height-2)` is
    not drawn: a window shorter than the rows would clip the draft (risk).
- Step 10, reconnect / offline / unavailable thread / draft handoff (`chat_composer/reconnect.rs`),
  both halves done (9fd0a4808):
  - Core `core/src/screens/composer/restricted.rs`: `Restricted::{Disconnected, UnavailableThread,
    DraftHandoff}`, `dispatches(text)` (what Enter may run: one line, `parse_slash_name` +
    `SlashCommand::parse`, an argument only for `supports_inline_args`; Disconnected runs only
    the four local commands `Warnings|Config|Quit|Exit`, UnavailableThread adds
    `available_when_thread_unavailable`, DraftHandoff runs nothing) and `expands_on_paste()`
    (only Disconnected: the app's Null key after a paste; the unavailable thread expands on the next key, a handoff keeps the placeholder). 5 core tests mapped from `reconnect_tests.rs`
    (`unavailable_thread_dispatches_recovery_and_local_commands`, `..._inline_commands...`,
    `disconnected_input_dispatches_local_config_command`, `restricted_input_preserves_blocked_drafts...`,
    handoff runs nothing).
  - Draft `src/widgets/composer_draft/restricted.rs`: `restrict(c)` (ends the history search with the
    draft as it was, clears the remote row selection, closes the mention popup, expands the pastes)
    and `expand_pastes(c)` (placeholders replaced by their text last first, elements after move with
    the change through `sync`, the caret keeps its place, images and mentions stay). 3 draft tests
    (`reconnect_expands_pastes_preserving_images_and_cursor` mapped; one shown biting: with the caret
    restore commented out two failed; a caret before a paste stays; `restrict` ends the remote row).
  - View (second half): `KeyData.restricted` and `Composer.restricted` (+ `PartialEq`, builder
    `Composer::restricted(mode)` for the page's UnavailableThread / DraftHandoff, not wired: their
    states are not in the app); `Disconnected` is derived in `build` (`SessionSlice.connections` of
    `agents.owner_of(thread)` is Reconnecting or Disconnected; never for `bare`). `take_key`:
    `restricted_key` first (restrict the draft, close the suggest popup; Enter, Tab and the bound
    submit/queue keys are not edits; the submit key tells `Submitted` only when `dispatches(text)`
    and no paste was expanded by that very key), and no popup, burst, history walk (`walk_history`
    is the caret's), remote row, Ctrl+R or composer_key while restricted. `ComposerMsg::Restrict`
    self-sent on the transition (flush the held burst, `restrict`, close the popup, tell the
    page); the paste handler expands on landing when `expands_on_paste`. Tests: 6 key tests in
    `editing.rs` `tests::restricted` (command passes / rest swallowed, handoff none, unavailable
    recovery, the expanding key runs none and the next does, Ctrl+U is an edit, Up is the caret's),
    7 page tests in `tests/pages/composer_draft.rs` `offline` through an injected
    `agents::Ev::Reconnecting` / `Reconnected` (paste expands where it lands; the loss expands a
    placeholder already there; Ctrl+U clears paste and text; Enter/Tab edit nothing and a prompt
    stays; `/status` stays; `/warnings` runs and takes the line; Up is no walk; whole again after
    `Reconnected`). The old test `the_history_walk_goes_on_over_a_submit_the_thread_did_not_take`
    (a Disconnected composer that walks history) is dropped: the oracle's offline Up is the caret's.
  - NEEDS (not mine, for the second half too): UnavailableThread needs a pub accessor for the
    thread's attachment (`core/src/threads.rs`); DraftHandoff needs a cloud-switch state in the app;
    the footer's "Ctrl+C quit" override is chat-status's; Left-key agents navigation in
    UnavailableThread is not built; the 6 screen-level reconnect tests of composer_4 stay pending
    (kit seeds for disconnected / handoff / unavailable); Ctrl+G stays pending G67.
- Step 7, Ctrl+R as a mode of the composer (2c4294dbe): the search lives in the composer
  because a paste reaches only a focused TextArea and the draft's `Snapshot` restores
  text, caret, placeholders and shell mode exactly. Core `screens/popovers.rs`:
  `HistorySearch` (pure: query, cursor, `Idle/Match/NoMatch`, `display_query` with `↵`/`⇥`,
  `step`, `set_query`, `preview`; 4 state tests in `core/tests/popovers.rs`).
  `widgets/composer_draft/search.rs`: `Session` (the search, the draft as it was, Vim's undo
  steps kept aside), `search_begin/edit/step/accept/cancel`, `search_rev` (the page draws
  again when it changes: the footer reads the draft, the page state holds a counter, R14).
  `pages/session/composer/search.rs`: the keys (`handle_history_search_key`: bound keys and
  Up/Down step, Esc/Ctrl+C cancel, Enter accepts a match only, Backspace/Ctrl+H/Ctrl+U edit,
  a character extends, every other key is consumed), the paste into the query (sanitized,
  whole), and the footer row (label dim, query info colour, `enter accept · esc cancel`
  keys bold words dim, `no match` red). The old screen-state path (`ChatMsg::Search`,
  `search_key.rs`, `Intent::Search*`, `Action::SearchPrevious/Next`) is gone. 5 page tests in
  `tests/pages/popovers.rs` (paste into the query, `↵`/`⇥` marks, cancel restores the draft
  with placeholder and caret for Esc and Ctrl+C, footer styles, a click in the preview), each
  shown biting once.

- Step 6, persistent history (pure core, ADR-003): the shared half and the per-composer half
  are split. Core `screens/composer/history.rs` (no rtwk, no IO), a split carry of the oracle's
  `ChatComposerHistory` and `search_batch.rs`:
  - `Store` (shared, changed only through `apply`; `popovers::Model.history: Shared<Store>`):
    the thread's log id and entry count, the entries and batches the host answered (`on_entry`,
    `on_batch`, `on_batch_error`), `rev` for polling. A reply with a stale `log_id` is ignored.
  - `History` (one per composer draft, `ComposerDraft`): the oracle's one offset space
    (persistent entries, then local ones), the walk cursor, the local submissions, replay seeds
    (`record_replayed_submission`), the last recalled text, `navigate_up/down`, `search`,
    `poll`. It never reads or writes the store: it takes `&Store` and puts the reads it needs
    into a `Vec<Lookup>` outbox (`Lookup::Entry{log_id, offset}`, `Lookup::Batch{log_id,
    end_offset}`), so a lookup is a value, not a call.
  - Ctrl+R as the oracle: probe the newest entry as one read, then bounded batches, dedupe by
    exact text, a failed batch retried `MAX_BATCH_READ_RETRIES` = 2 times, then `NoMatch`.
    `SearchStatus::Searching` is drawn in the footer (`searching history`, dim) until the
    reply comes; `Preview::{Show, Original, Keep}` says what the draft shows meanwhile.
  - Recording as the oracle: a submit records the expanded text with its image placeholders and
    local image paths; Ctrl+C records the placeholder text plus `pending_pastes` and writes no
    file; a search accept and a cancel reset the walk as `reset_search`/`reset_navigation` do;
    Vim Normal's last-character caret counts as the end (`vim_end`, `history_navigation_cursor`).
  - Outbox: `popovers::Ask::{Append(text), Lookup(Lookup)}` in `Model.asks`, drained with
    `take_asks()`; events `Ev::{History(seed), HistoryMetadata, HistoryEntry, HistoryBatch,
    HistoryBatchFailed}`; intents `Intent::{Record, Lookup}`.
  - View: `ComposerDraft` keeps `prompts: History` (the old `recalled`/`local` are gone);
    `ComposerMsg::{Lookups, Polled}` send the draft's lookups out through `cx.emit` and resume
    the walk when the store changed (`history_poll`, sent from `build` while waiting).
    `page/composer/{editing,vim,search,strings}.rs` use it.
  - Tests: `core/tests/composer_history.rs` (21: the oracle's history and batch tests carried,
    3 new: a late metadata keeps the replay seeds, a stale log id is ignored, local entries
    survive a metadata change), `core/tests/popovers.rs` (history search over the store, a
    Pending/Unavailable test), `tests/pages/composer_history.rs` (7: Up asks the host and the answer fills the draft,
    an answer for another log is ignored, Ctrl+R says `searching` until the host answers and
    then previews the match, a failed read restores the draft and says nothing, Ctrl+C keeps
    the draft for Up but writes no file, a submit is appended and found by Up, Vim Normal
    walks on from the last character); shown biting once (the lookup left out, the poll left
    out, the Vim end left out).

Counts now, end of step 11 (`summary.py --run` of the six shards): composer_1 8/1/41/0 of 50,
composer_2 31/4/15/0 of 50, composer_3 34/2/20/0 of 56, composer_4 18/2/53/0 of 73,
composer_popups 19/0/24/0 of 43, textarea 18/2/39/0 of 59: 128 pass, 11 n/a, 192 pending, 0 red
(109 pass before this job's Vim steps). Step 11 turned on: the 4 Vim normal history tests of
composer_popups (kit `keymap` seeds F9 = `global.toggle_vim_mode`, `vim_normal.redo` unbound,
`vim_normal.move_up/down` on F2/F3; two fixtures `h_first.jsonl`, `h_first_second.jsonl`), the
Vim preview test (the caret of a recalled entry or an accepted search match rests on the last
character in Normal mode, `rest_at_end`; Ctrl+R opens the search in Normal mode when redo is not
bound to it), `slash_init_dispatches_...` (the command exists now), and 2 of the 3 reconnect
tests (`fixtures/composer_4/offline.jsonl`: `agents` `reconnecting`). The stale pending reasons
(`no Vim mode`, `no image rows above the composer`, `no disconnected composer`) now say what is
really missing. `shards.py --check`: 0 stale.

Gap rows G-new-1, G65-G68 and G11 are in `rtwk-gaps.md`. Left: the embedded plain-text composer
(D4: a question sheet's answer composer, no sheet to host it yet).

## Binary run (end of chunk-composer, debug build)
`check.py --bin rtwk --bin oracle --only multi-turn,text-100 --capture-dir /tmp/cc-cap` (PTY): 4/4
pass (both binaries, the same composer rows at `ready` and after each turn; the rtwk multi-turn
sentinel is `no` as on the base). The restricted (offline) composer is not reachable from a mock
scenario: the mock model never drops the connection, so its keys are held by the page tests
(`tests/pages/composer_draft.rs` `offline`, injected `Reconnecting`) and the key tests in
`composer/editing.rs`. Differing rows: none seen in the two scenarios; the 3 composer_4 reconnect
screen tests: 2 green, `offline_delete_reveals_...` pending (the page's `Reconnecting…` row and
footer around the composer, no `↑` cue on a wheel-scrolled composer).

## Shard counts
See "Counts now, end of step 11" above (the third-run table that stood here was stale).

Reds on the base tip that this branch moved: 9 composer_2 and 15 composer_popups
tests were red on `claude/tui-rtwk` itself (checked first, as the checkpoint asked).
Fixed: `handle_paste_small_inserts_text` (paste sanitizing), the `$`/`@` skill
completion cases (separator space, punctuation, skills under `@`), the
`/mention` test (it asserted the composer alone; the app inserts the `@`). The rest
are pending with their reason (below), none loosened.

## Decisions
- Paste burst (composer-2, third run). Core `screens/composer/paste_burst.rs` is the oracle's
  `bottom_pane/paste_burst.rs` carried as it is (REUSE; `Instant` became a `Duration` on the
  app clock, R11; the oracle's 5 unit tests are kept). The view side: `ComposerDraft` holds
  the machine, `pages/session/composer/burst.rs` asks it before the field does (the
  `EditingKeys` door: `take_key` runs `burst::take`, then `edit_key`): a plain char is held
  9 ms (`RetainFirstChar`) or joins a burst, Enter and Tab inside a burst are `\n` and `\t` of
  the paste, a shortcut or navigation key flushes what is held first. What is held is
  flushed by an app timer (`AppBinding::start_timer`, `ComposerMsg::BurstDue`): every
  accepted key arms its own 9 ms timer, an early one finds nothing due. A real paste event
  flushes the held keys first and ends the window (`clear_after_explicit_paste`). The page's
  `?` and `Left` shortcuts of an empty prompt wait while `ComposerDraft::in_burst`
  (`is_in_paste_burst`). A `!` paste at the start enters shell mode before the text
  changes (`absorb_shell_prefix` sets the mode first: the command popup reads it on the
  change, a `!/diff` paste must not open it).
- Kit pacing (tests/kit/mod.rs, tests/common/mod.rs): the keys of a person are
  `KEY_PACE_MS` = 10 ms apart: `Screen::type_text` and now `Screen::keys` (token by token),
  and `Harness::press` waits 10 ms before the key (a `type_text` then `press("enter")`
  must not be one burst). `Screen::type_burst(text)` is a terminal without bracketed paste:
  a key per char 1 ms apart (`\n` is Enter, `\t` Tab), nothing waits after the last. The
  app draws one frame per 16.7 ms, so the screen right after `type_burst` shows the state
  at the first key: tests read it after `wait` (the oracle's "the draft is empty while the
  burst holds" is the core unit test of the machine). Raw `Tester::type_text` in other
  tests is one instant: `tests/services/shell_cycle.rs` got a `pump_for(50)` after its
  typing (the page's Ctrl+X chord does not flush a held burst) and
  `tests/pages/composer_draft.rs` waits for the shell command to reach the fake
  (`wait_idle` was true before it arrived: a flake of the first run).
- Placeholders are atomic elements through the controller (EditingKeys and a
  controller listener), the person's decision on chat-composer 2026-11-19. Not a gap.
- Shell mode (`is_bash_mode`) is UI state in `ComposerDraft`: a `!` typed at the
  start, or yanked there, becomes the prompt; Backspace at the start and Esc on an
  empty draft leave it; history recall of `!x` restores it; no command popup in it.
  Tab submits when idle, except a shell command. Shell input pastes stay literal (no
  blockquote quoting).
- composer-2: a `!cmd` line runs as the user's shell command (`submit_shell_command`), and
  only a draft in visible shell mode does (a pasted placeholder whose payload starts with
  `!` stays a model prompt: `composer::shell_line(line, shell_mode)` in core, unit-tested).
  Path (ADR-003, no RPC in a screen): `run_line` -> `chat::Intent::Shell` -> `app.rs`
  `apply_thread(ScreenThreadIntent::Chat(ThreadIntent::Shell))` -> `chat::apply_thread_intent`
  -> `Command::Shell` -> `chat::wire` -> `ThreadAsk::Shell` (`thread/shellCommand`,
  `core/src/threads.rs`; the rules test `thread_rpcs_only_in_threads` now names
  `ThreadShellCommand` too). `!` alone is the `Notice::ShellHelp` (the oracle's usage
  line). A turn running or not does not matter (`core/tests/composer_shell.rs`). No local
  echo: the result comes back as the server's item, as in the oracle. Tests:
  `composer_shell` (core, 3), `composer_draft` `a_shell_draft_the_thread_took_clears_...`,
  `a_bang_slash_line_is_a_shell_command_not_the_slash_command`,
  `a_bang_after_the_start_is_part_of_the_prompt`; shard composer_2
  `bang_prefixed_slash_text_submits_literal_shell_command` (its first form asserted the
  line as a prompt in the transcript; now the screen shows the cleared composer and the
  request is in the page test).
- composer-2: input-size limit. `composer::too_large(text)` (core, chars over
  `MAX_INPUT_CHARS` = 1 << 20) -> `Notice::InputTooLarge { provided }` (shared
  `core/src/commands/dispatch.rs`, two enum lines) -> `chat_strings.notice()` -> the notice
  above the composer; the draft stays. Both Enter and Tab (queue) go through `run_line`.
  The oracle's `set_steer_enabled` is a no-op (`_enabled`), so `oversized_queued_...` is the
  same Enter there; the port drives its Tab route. Both shard tests assert the placeholder
  row `› [Pasted Content 1048577 chars]`: the oracle's tests set the text with
  `set_text_clearing_elements`, which keys cannot reach (a paste that size is a placeholder).
  Bite shown: with the two checks taken out of `run_line` both tests fail.
  `composer_4 embedded_composer_shows_validation_error_...` stays pending: the
  `Message too long` flash belongs to the plain-text embedded composer, which has no entry.
- composer-2: the drag selection IS drawn, with the theme's `on indexed(237)` background,
  not REVERSED: rtwk's `TextArea::selection_style` (default the theme's `selection`) is the
  Flutter `selectionColor`; the composer sets `Style::new().attrs(Attrs::REVERSE)` as the
  oracle's textarea draws it. Not a gap (the earlier "not diagnosed" row text is gone).
  `selection_release_copies_and_keeps_the_editable_selection` passes;
  `selection_release_reports_only_clipboard_result` is n/a (the oracle's
  `copy_selection_on_release` / `finish_copy` closure API, no screen). Mouse tests:
  `composer_mouse` `a_double_click_selects_the_word_and_typing_replaces_it`,
  `the_drag_is_drawn_reversed_...`.
- Shell mode and the history walk end where the composer clears, not at Enter
  (`ComposerDraft::cleared`, called through `composer::clear_sent` by the page when the
  thread took the submit, by a command that took the line, and by Ctrl+C). A submit the
  thread did not take (no writer) clears nothing and keeps the `!` prompt and the walk;
  the page re-reads its draft flags after each clear and the `Composer` widget carries
  the shell flag it was built with, so its prompt follows. Tests: `composer_draft`
  `a_shell_draft_the_thread_did_not_take_stays_a_shell_draft` and
  `the_history_walk_goes_on_over_a_submit_the_thread_did_not_take` (red on the old code:
  `› ls` and `› two`), and `..._the_thread_took_clears_and_leaves_shell_mode`.
- A small paste whose text starts with `!`, put at the draft's start (caret or selection
  at offset 0, not in shell mode), is shell mode: the `!` becomes the prompt, the caret
  stays after the text (`insert_str_at_target` with `ShellPrefixHandling::Interpret`,
  then `sync_bash_mode_from_text`). A large paste is a placeholder first, as in the oracle.
  `a_pasted_bang_at_the_start_is_shell_mode` (red on the old code: `› !ls -la`) and
  `a_pasted_bang_after_text_stays_text`.
- The shell `!` prompt is bold (`light_red().bold()`); the footer's `Shell mode` label is
  not, so the bold is added at the prompt's call site and the `shell()` token is unchanged.
- Local history entries keep their placeholders in `ComposerDraft`. The core
  history stays `Vec<String>` (ADR-003: submits go through `run_line` / `Intent::Record`).
- Blockquote paste is a pure core rule (`blockquote_paste`): after `> ` a multiline
  paste is quoted line by line, counted for the large-paste placeholder, and the
  caret leaves the quote (`\n\n`).
- Pasted text goes through `sanitize_paste` (the oracle's `sanitize_user_text` after
  its CR/CRLF fold) before the large/quoted checks; a paste the sanitizing changed is
  inserted by the draft, an unchanged small one by the field.
- A mention completion's separator follows the oracle's
  `advance_past_completion_separator`: a plain space already after the caret, with
  only blanks behind it, is the separator (the caret steps over it and the
  completion's own space goes); else the new space stays and, for a skill, the next
  key may swap it for punctuation (`,.;:!?)]}` and the CJK marks).
- `@` lists skills too (the oracle's unified mention popup); a skill picked from
  `@` is inserted as `$name`.
- The textarea shard opens the debug entry `composer/textarea`, the bare TextArea.
  The debug menu under it marks its Column `overflow_intended` (one line in
  `src/app/debug/menu.rs`, the announced exception).
- Landing items outside this area: a `shell()` token in `src/app/theme.rs` (light
  red, the oracle's `light_red()`, not bold: the prompt adds the bold itself;
  startup-finish's area, the lander resolves);
  `[[test]] composer_mouse` in `Cargo.toml` (precedent: `agents_mouse`,
  `startup_mouse`); the session page's import order was formatted
  (`session_page.rs`, fmt only).

## Gap rows (docs/rtwk-port/rtwk-gaps.md)
- G65 (renumbered from G55: startup-wizards took G55, chat-cells G60-G64): TextArea
  wrap and caret at full rows, wide/half-width graphemes, hanging spaces and tabs, rect
  clearing and scroll (21 textarea tests).
- G66: Ctrl+J arrives as Enter (`key_combo.rs`), so no Ctrl+J newline; the tests break
  lines with Alt+Enter.
- G67: no widget-level terminal suspend for `$EDITOR` (Ctrl+G external editor; one
  composer_4 test).
- G68 (still a gap): the popup is `Popover` + `OptionList`; the oracle's rows are aligned
  name/description columns with a `›` marker and an `enter insert · esc close` line, rtwk's
  list has a `▎` bar and a right-aligned detail (5 composer_popups and 2 composer_2 tests).
- G69, G70 (closed, route (b)): `Autocomplete` takes Tab/Enter alike and has no mouse, so
  the composer owns `suggest.rs` (`Suggest` model read by the `EditingKeys` hook, drawn by
  `Popover` + `OptionList`): Enter picks (a command runs, a mention is put in), Tab completes
  `/name ` as text, a typed `/` completes the highlighted command, Up/Down/Ctrl+P/Ctrl+N and
  the wheel move the highlight (wrapping), Esc closes until the text changes, a click picks
  the row, a press outside closes. Page tests `tests/pages/composer_suggest.rs` (10); bite
  shown: with the Tab arm off, 4 of them go red. Shards un-pended: composer_2 the two
  slash-tab tests and `slash_key_completes_selected_slash_command_as_text`, composer_4
  `paste_burst_tab_does_not_accept_a_completion`; the click test of `composer_mouse` runs.

- G68 route 2 not tried: the popup rows composed in the layout under the field from
  `Row` / `SizedBox` name and description columns, a `›` `Text` and the hint line; the
  oracle's look may need no rtwk change, the row stays until someone builds it.
- Review fixes (phase 2): a pick rewrites only the word being completed (an image or large
  paste before a `$`/`@` mention keeps its element and goes with the turn; page tests
  `a_mention_after_an_image_keeps_the_image`, `a_click_on_a_mention_row_after_an_image_...`,
  bite shown: with the old whole-prefix rewrite both fail); Tab on `/skills` runs it as Enter
  does (oracle `selected_command_dispatches_immediately_on_tab`).
- Not built, from the review: history recall does not restore images (`popovers.history` is
  text only, shared with Ctrl+R search and the persisted history; chat_live_5
  `ctrl_c_cleared_prompt_is_recoverable_via_history` is pending again with the image check
  in its body); the popup's pick drops no draft tail but does not keep it as inline
  arguments the oracle's way (`complete_selected_slash_command_preserving_existing_draft_tail_as_inline_args`);
  the startup queue replays every item at the thread start, the oracle drains one item per
  finished turn (the order is the same, a queued `!cmd` runs while the message before it
  runs: the same rule as a shell during a turn); images are sniffed as PNG, JPEG, GIF and
  WebP only (the oracle's `image_dimensions` reads more formats).

## Pending and why (the rest, none built here)
- Paste burst, what stays pending: composer_3 `shortcut_modified_input_does_not_enter_paste_bursts`
  and `non_text_input_flushes_pending_paste_without_inserting_text` (the kit's `keys()` has no
  SUPER modifier: the kit does not enable the kitty keyboard protocol),
  composer_3 `plain_and_shift_spaces_insert_normally` (the oracle's snapshot is the composer
  alone with its own `100% context left` footer; the typing is the page test
  `composer_burst a_space_inside_a_burst_is_kept`),
  composer_2 `ascii_burst_treats_enter_as_newline_even_when_parent_owned` (no parent-owned
  thread seed), composer_4 `paste_burst_tab_does_not_accept_a_completion` (built with the popup, see G69).
- Images `[Image #N]` (built 2026-11-21): a pasted image path (quotes, `file://`, escapes,
  Windows paths; the file must sniff as PNG/JPEG/GIF/WebP) becomes an atomic element in
  `ComposerDraft` (numbered by attach order, relabelled on every edit, a yank gets a new
  number); submit carries `UserMessage { text, images }` through `Intent::Submit`,
  `ThreadEv::Submit` and `Command::StartTurn` to `protocol::turn_input` (`LocalImage`
  items first, the text with its `text_elements`). The kit seeds it (`attach_image`,
  `image_file`); tests `composer_images`. Not built: clipboard image paste (Ctrl+V needs
  the clipboard-image dependency and IO), `/plan <args>` carrying the images, remote
  image rows and the thumbnail strip (transcript_view_1).
- Startup draft (built 2026-11-21): typing while the session is `Starting` works (the
  draft is the page's, not the thread's); Enter before the first thread queues the message
  or the `!cmd` in order in `CodexApp` (`BeforeThread`), and both go out when the thread
  starts (`composer_startup`, core `model.rs`). Not ported: a slash command typed before
  the start is answered by `dispatch`'s notice (`SessionStarting`), the oracle queues it
  (`QueuedInputAction::ParseSlash`) and parses it when configured; the empty-state
  animation dismissal; `thread_stream_disconnected` / rate-limit-recovery queue pauses.
- Vim mode, reverse history search (Ctrl+R), shortcut chords from the keymap, rebindable
  queue key (`Ctrl+Q`) -> the keymap task / chat-nav-pane.
- Footer modes (`100% context left`, flashes, the composer-alone 60-column snapshots) ->
  pane-footer.
- `/init` (no handler) and `/diff` (the pager is not built) -> chat-nav-pane / pager.
- Embedded, disconnected and draft-handoff composers, plugin mention bindings, remote
  image rows: no seed on the chat screen.
- Markers whose body passes (27 of the 248, found by the probe, `KIT_PROBE_PENDING=1
  summary.py --run <the six shards> --list stale`: composer_2 3, composer_3 5, composer_4
  13, composer_popups 4, textarea 2). The table's `stale 0` is the normal run, where a
  marker fails first, so it cannot show them; `shards.py --check`'s "0 files stale" is a
  different check (the shard files against the oracle's sources). The markers stay: each
  body asserts only the half the oracle's test has that the port can draw (the feature
  itself is missing: file search, remote images, handoff, Vim, mention bindings, keymap
  config, links), so a pass would be a false one. Two were corrected in this round because
  they were stale for real: `editor_actions_insert_shell_prefixes` (shell mode and Ctrl+K /
  Ctrl+Y are built; only the Vim part `r!` is missing, and the reason now says so) and
  `composer_mouse_edits_flush_typing_...` (the click places the caret). Nine more reasons
  that still said "no large-paste placeholder" or "no mouse selection" were reworded to
  what the probe shows now (images, Vim, file search; the drag selects and is drawn REVERSED now).
- The textarea mouse tests (`MOUSE` in textarea.rs) stay pending: their rows are G65's and
  they need rect offsets (a 1-row rect at y 1, a resize mid-test) the kit cannot seed.

## Unit tests
- `core screens::composer::paste_burst::tests` (the oracle's five), page tests
  `tests/pages/composer_burst.rs` (9: Enter in a burst, `?` in a burst, `?` alone, Left
  behind a held char, a lone char after 9 ms, a space in a burst, a space at a person's pace, `keys("h i enter")`
  submits, a large burst is one placeholder). Bite shown once: with the page's
  `in_burst()` check off the `?` and Left tests fail; with the Enter arm off the Enter test
  fails.
- `composer::editing::tests`: the oracle's kill and yank state tests, plus the atomic
  placeholder keys. Bite shown: breaking the EOL kill and the yank turns them red.
- `widgets::composer_draft` tests (moved with the file).
- `core screens::composer::tests`: `a_message_over_the_limit_is_too_large_by_its_characters`,
  `only_a_draft_in_shell_mode_runs_a_bang_line`; `core/tests/composer_shell.rs`.
- `core screens::composer::tests::sanitize_paste_*`: the oracle's `sanitize_user_text`
  cases and the `_count_rows` paste.

## Mouse (ADR-011)
`tests/pages/composer_mouse.rs` (kit mouse events on the live chat screen): a click puts
the caret in the draft (`› helloX world`), a drag selects and the next typed text replaces
it (`› hello X`). Bite shown: without the click and the drag the rows read `› hello worldX`.
A click on a popup row picks it (`composer_mouse`, `composer_suggest`). In the oracle shards: composer_4
blockquote_paste_replacing_selected_*, backspace_deletes_a_body_selection_*, composer_3 a click
inside a placeholder snaps the caret to its end, textarea target_replaces_*.

## Binary run (composer-2 third run, paste burst in; debug build)
`check.py --bin rtwk --bin oracle --only multi-turn,interrupted --capture-dir /tmp/composer2-cap`:
4/4 PASS again (multi-turn 15.0 s rtwk / 14.0 s oracle, interrupted: Esc disconnects the
stream in 0.47 s rtwk / 0.33 s oracle). The scenario's typed text and Enter go through the
paste detector in the real binary and the three turns still submit. The text of the previous
run below still holds for the rows.

## Binary run (fix round, debug build)
`check.py --bin rtwk --bin oracle --only multi-turn,interrupted --capture-dir /tmp/composer-cap`:
4/4 PASS (type and submit three turns, 15.0 s rtwk / 14.3 s oracle; Esc interrupts a
streaming turn in 0.27 s rtwk / 0.45 s oracle and the next prompt works). The captures are
raw ANSI, not a rendered grid, so no row was compared from them: the composer rows are
held by the oracle shards. Known difference in the same screens: the oracle's footer is
`? for shortcuts` plus the model/cwd status line, ours is the page's key hints
(`← for agents · ? for shortcuts`, and `tab to queue message` while a draft waits); footer
modes are pane-footer's. No pty scenario types `!`, pastes, queues or uses the mouse yet:
those are covered by the shard tests above, not run in the binary.

## Environment notes
- `~/.cache/rtwk/sccache-main/bin/sccache` was missing on 2026-11-21, so `acargo` failed;
  the checks ran through a copy of `acargo` without `RUSTC_WRAPPER` (same flags, own
  target). An earlier disk-full (ENOSPC) was cleared by removing the scratch worktree
  `/tmp/composer-base` this task had made for the base check.

## Binary run (composer-2 phase 2, popup in; debug build)
`check.py --bin rtwk --bin oracle --only multi-turn,interrupted --capture-dir /tmp/composer2-cap`:
4/4 passed. A pty drive of the rtwk binary (keys one by one, 100x24, ANSI stripped; no screen
emulator, so the diff-rendered stream is read, not a grid): `/stat` opens `▎/status  show
current s...`, `/` lists the commands (`/model`, `/ide`, `/permissions`, `/keymap`...), two
Downs move the bar to `/ide`, Esc closes the list, a typed `/stat` and Enter print the status
card (`Model: gpt-6-sol  Directory: ...`). Tab, startup draft and images were run in the page
tests only. Differing rows against the oracle: the G68 layout (`▎` bar, right-aligned detail,
no hint line); the rest equal in what was driven. A `/stat` plus Enter written in ONE write is a
paste burst in both binaries (Enter is a newline), as the oracle does.

## block-composer: chunk-composer-live-mentions and chunk-composer-history (2026-11-23, branch fleet/block-composer, tip 2054483f3, landed in 0130f3747)
Vim (fdadb68ff) and images (cc0bbabd5) of the same block were not on the branch at batch 4 (51abab645); chunk-composer-states was not taken.

### Live mentions (b2109d3ab feature, f6fc34e15 merge, 072030ea7 fmt; mention-seeds earlier 7204119df)
- The `@` and `$` popups list the server's catalog: `core/src/screens/composer/mentions/sources.rs` (skills, plugins, apps become `Mention`; 4 unit tests); the popovers model has `Listed`, `Ev::Catalog` and a per-thread `ThreadEv::Load` (`skills/list` and `plugin/installed` through `ThreadFx`, once per thread once its cwd is known, the writer only); `core/src/app/mentions.rs` sync (hook in `apply_app_from`; `app.rs` has `mod mentions;` and `mentions::sync(app, fx);`); the installed apps of the /apps cache join the catalog. 5 core tests (`core/tests/popovers.rs` `mod live`).
- Checks: core suite 1537, gate nextest 3059, rules 80, clippy and `shards.py --check` clean; fmt fixed in 072030ea7 (the gate's only failure).
- Key oracle tests green: composer_3 `file_completion_preserves_large_paste_placeholder_elements`, `file_completion_for_sigil_path_does_not_reopen_popup` (un-pended with fixtures `file_ma_main.jsonl` and `file_ma_sigil.jsonl`), `dismissed_file_popup_*` (2); composer_2 `reopening_identical_unified_mention_restarts_file_search`; composer_popups `long_paths_keep_distinguishing_filenames_and_original_selection`. Shards: composer_2 38 pass / 4 n/a / 8 pending, composer_3 41/2/13, composer_popups 27/0/16, composer_1 24/5/20; no red from this work.
- n/a (MentionsV2 off, the person's decision): composer_1 `file_popup_ignores_{shell_positional_parameter,bare_shell_parameter_with_matching_skill,unbindable_qualified_skill}_snapshot` and `plugin_mention_popup_snapshot` (the area was widened to composer_1.rs).
- Sweep: composer_3 `adjacent_image_completion_inserts_leading_separator` (needs image attach and Vim: chunk-composer-images / chunk-composer-vim); the other pending composer_2/3/popups tests belong to chat-live and chat-nav-pane chunks (agents hint, Vim, remote images, history, shutdown/input-disabled, the G68 popup layout).
- Binary beside the oracle (pty 100x24, mock model): the `@sr` and `$sk` popups are identical row for row (Skill Creator / Skill Installer, header, tabs, hint line). Differing row, not this chunk: the status line under the composer wraps in ours (`GPT-6-Sol default fast · /private/...` over two rows, the oracle `GPT-6-Sol default · ...` on one) -> status-line chunk.
- Gaps: (1) the `@` file search runs through the host's in-process search (`Ev::FileMatches`); routing through the app-server `fuzzyFileSearch` is not ported and was not needed. (2) The apps cache loads only when /apps opens (`asked_apps` in `select.rs` is private), so `$` app mentions appear after /apps; the oracle loads connectors at startup. Need for the select/apps owner. (3) The oracle's skill-load warnings (invalid SKILL.md after skills/list errors) are not ported.
- Mouse: the popup's row click and the tap on a tab already have rtwk_test tests (`tests/pages/composer_mentions.rs` `a_click_on_a_row...`, `a_tap_on_a_tab...`); no new selectable view.
- Pre-existing red, not from this work (red with the hook disabled too): composer_1 `deleting_prefix_keeps_bang_literal_on_submission`.

### History across sessions (d4b66e1bc feature, 2054483f3 merge)
- Most of the chunk was already in the tree (persistent recall through the host, slash recording in `actions.rs` `record`/`recorded`, replay seeding in `session_page.rs` and `live/restore.rs` `seed_history`, `ComposerDraft::replayed`); its pending reasons were stale. Done here: `Prompt::from_inputs` is `pub(crate)` and the copy `prompt_of` in `replay.rs` is deleted (`core/src/screens/backtrack.rs`, `core/src/threads/replay.rs`).
- Oracle tests un-pended and green: composer_3 `bare_slash_command_can_be_recalled_after_recording_pending_history`, `popup_selected_slash_command_records_canonical_command_history` (both close the `/diff` overlay with Esc first: until the pager lands `/diff` shows the not-yet popup, which takes the Up; the oracle's real overlay is closed the same way); composer_2 `remapped_history_search_does_not_fall_back_to_ctrl_r` (the keymap seed is read now).
- Page tests `tests/pages/composer_history.rs` `mod resumed`: `up_walks_the_replayed_prompts_newest_first_and_down_walks_back`, `a_prompt_sent_in_this_session_comes_before_the_replayed_ones`. The core replay-seed test (`core/tests/app_wiring.rs` `replayed_user_messages_seed_the_composer_history_oldest_first`) and the oracle chat_live_2 `replayed_user_messages_seed_composer_history` were green already.
- Checks (scoped): core replay/backtrack/history 97 green, pages composer_history and composer_mentions green, 5 key oracle tests by name green, rules 110 on the merged tip, clippy and fmt clean, `shards.py --check` clean.
- Binary beside the oracle (pty 100x24, mock model, shared CODEX_HOME across two runs): submit `remember-me`, quit, restart, Up gives `› remember-me` in both rtwk and the oracle. Differing rows, not this chunk: the status line under the composer (ours `GPT-6-Sol default fast · <cwd>` against the oracle's `GPT-6-Sol default · <cwd>`; the oracle's `warning · f2 to view` footer shows 1-2 warnings where ours shows none on the restart) -> status-line and warnings chunks; during the running turn the transcript tail differs (ours shows `Working` and the draft area, the oracle returns the empty composer at 6 s) -> chat-live.
- Sweep: composer_3 `history_navigation_restores_remote_and_local_image_attachments`, `history_navigation_restores_remote_only_submissions`, `left_respects_editor_and_vim_history_remaps`, `vim_normal_history_navigation_from_start_of_bang_command_recalls_older_entry` (need images and Vim: chunk-composer-images / chunk-composer-vim).
- Mouse: no new selectable view. Needs: none.

## block-composer: images and Vim (2026-11-23, branch fleet/block-composer, fdadb68ff, landed in batch 5: de6ff75d3, land e85db56bd)
Sources: the commit messages and diffs of cc0bbabd5 and fdadb68ff, the batch 5 land commit (land-check green), the oracle test files after the merge. No new pty run was made for this section; the tm task summaries of the two chunks could not be read when this was written, so nothing below goes beyond the commits.

### Images (chunk-composer-images, cc0bbabd5)
- Remote image URLs travel with the message: `UserMessage.remote_images` go into `turn_input` before the local images, through `merge_messages`, `Draft::message` and `put_back` (`core/src/protocol/mod.rs`, `core/src/threads.rs`, `core/src/threads/message.rs`, `core/src/screens/backtrack.rs`).
- Enter and Tab on remote images alone submit (`src/pages/session/actions.rs`, `composer/mod.rs`); a recalled history entry repaints its remote rows (`composer/editing.rs`, `live/restore.rs`); a selection that cuts an element grows to hold it (`widgets/composer_draft.rs`).
- Clipboard images (Ctrl+V, Alt+V) go `Intent::PasteImage` -> `Ask::ClipboardImage` -> `src/services/effects.rs` (`paste_image`) -> `Ev::ImagePasted` (this flow came with chunk-composer step 9, 4e177d37f); the kit answers the ask for tests.
- Kit seeds (`tests/kit/seed.rs`): `Open::remote_images`, `Screen::remote_images`, `clipboard_image`, `paste_clipboard_image_at_once`.
- Oracle tests un-pended with the seeds (found by comparing the `kit::pending` calls of each test file before and after the commit): chat_commands_1 `enter_with_only_remote_images_submits_user_turn`, `shift_enter_with_only_remote_images_does_not_submit_user_turn`, `submission_with_remote_and_local_images_keeps_local_placeholder_numbering`; composer_2 `remote_image_rows_snapshots`, `slash_plan_args_preserve_text_elements`; composer_3 `delete_selected_remote_image_relabels_local_placeholders`, `deleting_reordered_image_one_renumbers_text_in_place`, `history_navigation_restores_remote_and_local_image_attachments`, `history_navigation_restores_remote_only_submissions`; composer_4 `composer_mouse_edits_flush_typing_and_reconcile_placeholders`. The reason "no seed for a clipboard image (alt+v) after a held `!`" was also removed from a chat_commands_1 test (the test still holds another pending call or was rewritten; not checked).
- Sweep: composer_3 `adjacent_image_completion_inserts_leading_separator` (still pending: no file completion with an image attach and Vim Normal `u` in the kit); composer_4 bodies whose reasons still name remote image seeds (remote rows beside mouse selection; selecting and deleting a remote row as one Vim edit, `tests/pages/composer_images.rs` holds the rows).
- Missing: a run of the clipboard flow with a real clipboard in the binary.

### Vim (chunk-composer-vim, fdadb68ff)
- `/vim` turns modal editing of the draft on or off and says so (`NoteKey::Toggled`, `core/src/threads/note.rs`, `core/src/app/session.rs`); F9 (`global.toggle_vim_mode`) says so too.
- `tui.vim_mode_default` starts a new thread's draft in Insert mode (`TuiSettings`, host config in `host/src/effects.rs`, `core/src/app/facts.rs`; the batch 5 merge keeps `copy_on_select` and `vim_mode_default` side by side).
- The page rebuilds when a key changes the Vim mode, so the footer label tracks it (`session_page.rs`, `thread_ui.rs`, `strings.rs`).
- A held or burst-flushed character is recorded for dot repeat (`composer/burst.rs`, `composer/vim.rs`, `widgets/composer_draft/vim.rs`). A bang in Normal mode enters Insert.
- Kit: `Open::tui` (`tests/kit/env.rs`), a `vim()` screen in the composer tests. Page tests: `tests/pages/composer_vim.rs`.
- Oracle tests un-pended (same comparison): chat_commands_1 `startup_draft_handoff_keeps_vim_insert_mode_for_nonempty_drafts`; chat_live_4 `vim_mode_default_enabled_starts_composer_in_insert_mode`; composer_1 `bang_enters_shell_mode_in_vim_normal_mode`, `empty_vim_insert_escape_enters_normal_without_esc_hint`, `esc_switches_vim_insert_to_normal`, `shell_command_can_be_typed_after_vim_normal_bang`, `slash_command_dispatches_after_vim_insert_slash`, `slash_opens_command_popup_in_vim_insert_mode`, `vim_mode_resets_to_insert_after_queued_submission`, `vim_mode_resets_to_insert_after_submission`, `vim_mode_stays_insert_after_suppressed_submission`; composer_3 `vim_normal_history_navigation_from_start_of_bang_command_recalls_older_entry`, `vim_normal_operator_pending_consumes_submit_key`.
- Sweep (still pending, reasons in the test files): composer_3 `left_respects_editor_and_vim_history_remaps` (no keymap seed for `editor.move_left`, `editor.move_up`, `vim_normal.move_left`); composer_4 Vim bodies (the kit toggles Vim with F9 but the bodies are not written for it: Replace mode, the `/` search query, Ctrl+R on an empty history, redo); approvals_1 and chat_approvals_2 (Vim in the question answer field, not connected).

### Checks and gaps
- Land-check green on the batch 5 merge candidate (e85db56bd); the chunk's own checks are in the commits' tm summaries (not read here).
- Needs: none recorded in the commits.
