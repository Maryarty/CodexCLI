# chunk-commands — parity
Chunk `chunk-commands` of docs/rtwk-port/research/briefs/chat-commands.md (§6). Partial pack, extended at each step.
Shard counts (summary.py, oracle profile; pass / n-a / pending / todo / red):
start (claude/tui-rtwk af5799546): chat_commands_1 0/15/35/0/0 · _2 4/9/31/0/2 · _3 1/14/50/0/0.
step 1:                              chat_commands_1 0/15/35/0/0 · _2 7/9/30/0/0 · _3 3/14/48/0/0 (0 red, 0 stale).
step 3:                              chat_commands_1 1/15/34/0/0 · _2 8/9/29/0/0 · _3 10/14/41/0/0 (0 red, 0 stale).
step 4 (no shard test turned on):    unchanged.
step 5:                              chat_commands_1 1/15/34/0/0 · _2 8/9/29/0/0 · _3 13/14/38/0/0 (0 red, 0 stale).
Whole nextest at step 5 (`--workspace --no-fail-fast`, 2135 tests, machine loaded): 5 reds, none left: `session_sse::embedded_server_resolves_exec_approval` and
`standalone_escape_disconnects_the_live_stream` (SIGABRT at 72 s / 162 s under load; pass alone), `shell_agents::typing_a_word_asks_the_server_once` (known flake),
`rules layers::the_shown_thread_is_set_only_in_apply_app` and `r14_widget_state_fields_hold_no_model_types` (mine: the removal answer cleared the shown thread in
session.rs, a comment named a model type; fixed: the clearing is in app.rs, the comment reworded; the rules pass).

## Step 1 — notes as transcript items, record rule (oracle history_cell/notices.rs:334-357, chat_composer.rs:3432-3563)
- Core: `threads/note.rs` (`Note { tone, key }`, `NoteKey::Command(Notice)`, `Tone::{Info, Error}`; `From<Notice>`: two answers are information
  — `ModelStartup`, `ShellHelp` — every other is an error, as `add_info_message` / `add_error_message`), `Item::Note` in the transcript,
  `chat::Intent::Note { thread, note }` (replaces `Notice`/`ClearNotice`; `chat::Model.notes` holds the answers given before any thread exists),
  `session::session_note` decides the place: an entry of the thread through `apply_thread(ThreadEv::Appended)` (ADR-003, one writer), else the screen's list.
- Composer rule (`Notice::keeps_draft`, `Notice::is_recalled` in core/src/commands.rs): the composer keeps the line of an unknown command, a message over the
  limit, a command refused while a task runs (`reject_slash_command_if_unavailable`) and a command dispatch validates (`/fork`, `/review`); the
  others take the line. Kept lines are in local recall (`ComposerDraft::recorded`, the oracle's `record_pending_slash_command_history`), an unknown
  command and a too-large message are not (`unrecognized_slash_command_is_not_added_to_local_recall`).
- View: `cells/note.rs` `NoteCell` (info: dim bullet + message; error: `■ ` in the error colour), `NoticeLine` draws only the screen-level list; words from
  `strings::note`.
- Tests: core/tests/commands.rs `notes` (4); oracle s2 `usage_error_…`, `usage_command_with_invalid_view_…`, `unrecognized_…`, `unavailable_slash_command_…`,
  `signed_out_usage_*` (two, seed fixture `signed_out.jsonl`), s3 `slash_keymap_invalid_args_show_usage`, `slash_mcp_invalid_args_show_usage`; page tests in
  tests/pages/commands.rs (3): the answer is a transcript row that stays above what follows, an information note is a bullet row, a command refused while a task
  runs runs once and keeps its line for recall. Bitten: the `/review`-recall oracle test was red until `ComposerDraft::recorded`; the shell-help page test was red
  (Enter on a bare `!` took the "Enter on an empty composer" path) until the guard in `ChatMsg::Submit` counted shell mode.
- Re-pended with a true reason: `interrupted_merged_message_history_encodes_mentions_once` (was red at the tip: the steered message waits in `pending_steers` and
  no row shows it; chunk-live's pending_input_preview owns it).
- The shard bodies' `esc enter enter` of kept-draft commands is `esc enter` (the oracle submits once; a second Enter on a kept draft runs the line again).

## Step 3 — the commands that run at once (oracle slash_dispatch.rs, chatwidget.rs:1551, 1741; chat_composer.rs:515, 3405)
- Core: `NoteKey` gains typed answers (`WorkingDirectory`, `Rollout`, `DesktopOpened`, `StoppingTerminals`, `Stub(Stub)`, `RawOutput`, `ParentOwned`) and
  `NoteKey::tone` (info or error as `add_info_message` / `add_error_message`); `Note::from(NoteKey)`. Words in `strings::note` (ADR-010).
- `session_intent` is exhaustive (no `_`): `/stop` -> `ThreadIntent::CleanTerminals` -> `chat::Command::CleanTerminals` -> `ThreadAsk::CleanTerminals`
  (`thread/backgroundTerminals/clean`, ADR-003: the RPC is a ThreadAsk), then the note; `/pwd` (thread cwd, else the status line's), `/rollout`
  (`ThreadPart.path` from `thread/started`), `/app` (`Command::OpenUrl codex://threads/<id>` + note), `/init` (bundled prompt as a message through `submit`),
  `/raw` (`chat::Model.raw_output`, `Intent::RawOutput`, note), `/debug-m-drop|update` (stub error). Foreign intents are named in one arm, each a later step or
  another chunk's, answering `NotAvailable` until built.
- Parent-owned thread: `ThreadPart.parent_owned` from `thread/started` (`can_accept_direct_input`, else a spawned sub-agent: `thread_blocks_direct_input`);
  the composer refuses everything but the allowed commands (`commands::parent_owned_takes`, oracle `handle_parent_owned_submission`: by the command's own name,
  alias refused) and keeps the line; `threads::apply` refuses `Submit` and `Compact` of such a thread (`submit_op`) so no path can start a turn.
- Kit: `Screen::commands()`, `Open::held_start()` (the start of the thread held: no thread id, `fake.hold_thread_start`). Fixtures `chat_commands_3/{side,rollout,parent_owned}.jsonl`.
- Tests: core/tests/commands.rs `effects` (12), page tests `raw_toggles_…`, `init_sends_…`; oracle s3 `slash_stop_…`, `slash_clear_is_disabled_…`,
  `slash_memory_maintenance_…`, `slash_app_without_thread_id_…`, `slash_pwd_and_cwd_…`, `slash_rollout_…` (2), s2 `no_op_stub_…`, s1 `parent_owned_thread_blocks_all_…`.
- Still pending, true reason: s1 `parent_owned_thread_blocks_settings_shortcuts` (shift+tab / alt+. / alt+, have keymap rows and no action in the port; no chunk of this wave
  owns them).
- Known hole, not mine: `Command::OpenUrl` (banner links and `/app`) has no executor in host/src/in_process.rs `work` (it falls to `answer`, which sends nothing), so the
  browser / Desktop app is never opened and `/app` cannot report a failure (`desktop_thread_open_error_message`). Handover: an executor with `webbrowser::open` (the
  startup seam `host/src/startup.rs:98`) and an answer for `/app`.
- Gap for the transcript chunk: `/raw` flips `Model.raw_output` and says so; the transcript's raw rendering (`cells/raw.rs` of chunk-cells) does not read it yet.

## Step 4 — /cd (oracle chatwidget/working_directory.rs:22-60, app/event_dispatch.rs:374-418, app/working_directory.rs:127-)
- Core: `session::change_directory`. Refusals as `NoteKey::CdRefused(CdRefusal::{NotStarted, NotIdle, HistoryNotSaved})` (error tone, words in
  `strings::note`): no thread yet; not an idle primary session (a turn running or about to start, a queued message, a pending steer, a side conversation,
  a parent-owned thread, an ephemeral one: `can_change_working_directory`); history that has no rollout path ("Conversation history is not saved.").
- Otherwise the session goes on in the directory through `AppCtx::NewConversation`, as `/new` and `/fork` do: a fork (`ThreadAsk::ForkAt{cwd}`) when the
  thread has history (the oracle's `fork_thread_for_directory_change`), a fresh start (`ThreadAsk::Start{cwd}`) when it has none. `commands::resolve_directory`
  expands `~` (`agents.home`), joins a relative path to the thread's cwd and folds `.`/`..` (no file system read: core has none).
- Tests: core/tests/commands.rs `directory` (7: the table of states, not started, fork with history, fresh start, history not saved, the new conversation takes
  the screen for Forked and Started, path resolution); page tests `cd_starts_the_session_afresh_in_the_directory`, `cd_in_a_conversation_with_history_…`.
- Oracle s3 `slash_cd_rejects_pending_input_…` stays pending with a true reason (its second half is the idle session with a queued message; the port drains the
  queue at turn end and refuses `/cd` while a task runs, so no key sequence reaches it); the whole table of states is the core test. s3 `slash_cd_changes_…` is n/a.
- Not built, named (no chunk of this wave owns them): the directory check of the oracle ("Not a directory: …", "Cannot access directory …": a `Workspace`
  method and a command for it in src/services/agents/workspace.rs, host-services), the remote-workspace refusal (needs a fact from the host), the unified-exec
  process and MCP-inventory-loading refusals (core holds neither state), trust confirmation of an untrusted directory, the permission-profile checks.

## Step 5 — /archive and /delete (oracle slash_dispatch.rs, chatwidget/slash_commands.rs, bottom_pane/selection_view `confirmation()`, app/thread_routing.rs `return_to_agents_after_removal`)
- Core: `select::Menu::Confirm(Confirm::{Archive{running}, Delete})` (2 rows), `Picked::Confirmed(Confirm)`; `Pick(1)` (Yes) logs the pick and closes, `Pick(0)`
  (No) closes with nothing. `session_intent` (`Archive`, `Delete`) opens the menu (`running` = the thread has a turn); `confirmed` (after every select intent in
  `apply_app`) drains the picks and `remove_current_thread` asks the agents backend (`agents::Command::Archive|Delete`, an app-scoped command, not a thread RPC:
  ADR-003 unchanged) under `AppCtx::Removal{thread, action}`. No thread: "… must start before …" note; a side conversation: refusal note; a second Yes while the
  first is unanswered asks once. The answer (`removed`): `Done` forgets the thread (`ThreadSet::forget`) and the `apply_reply` arm in app.rs (the one place that sets the shown thread,
  a rules test) sets `session.active = None` and `SessionSlice.removed += 1` when it is still the chat's; anything else is a `NoteKey::Remove(action, RemoveNote::Failed)` error note in the thread's transcript. A disconnect drops the
  outstanding removals of its origin.
- View: the sheet is the select sheet (`MenuDialog`) opened centered: `sheets::select::open_centered` (`DialogOptions{alignment: CENTER}` over `sheet_options()`),
  the picker wrapped in `Padding(symmetric(2, 0)) > Dialog::bare(picker).max_width(72)` for a `Menu::Confirm` (the oracle's `ViewPresentation::Centered`, `dialog_area`:
  72 wide at most, 2 cells in). `pages/session/actions.rs` emits the model's question (`chat::Intent::Session`) and opens it with `open_confirmation`
  (`last_opened()` stays `Target::Select`). `SessionPage` opens the command center (`Target::Agents`) when `SessionSlice.removed` grows and the chat is the route in
  front (`Route::is_current_of`); under another sheet it waits until the chat is in front. Words: `sheets/select/strings.rs` (confirmation), `pages/session/strings.rs` (notes).
- Tests: core/tests/commands.rs `removal` (10: the menus, Yes asks, No leaves, a pick is not left for later, no thread, side conversation, the server removing
  the thread, a refusal, the second Yes); page tests (tests/pages/commands.rs, 4, incl. the mouse: a tap on "Yes, archive" asks the archive once and closes the sheet,
  a tap on "No, keep this session" asks nothing, `Done` opens the command center, a refusal is the error note and the chat stays); oracle s3
  `slash_archive_cancellation_keeps_task_running`, `slash_archive_confirmation_requests_current_thread_archive`, `slash_delete_confirmation_requests_current_thread_delete`.
  Bitten: `threads.forget` removed (core test red), the `open(cx, Target::Agents)` trigger removed (page test red).
- The snapshots are the oracle's bottom pane only (the composer shows through left of the dialog: "› As", "  GP", "• Wo", which a full frame has no row for), so the
  three tests compare the rows of the dialog (cols 4.., the snapshot's rows with the four backdrop columns blank) at the centered position of the 80x24 screen
  (`(24 - rows) / 2`), the way chat_live_3.rs does for the review popup; the rows equal the oracle's, wrapped descriptions of `/delete` included.
- Not carried: the saved side chats of "Also deletes subagents and saved side chats" (no `/side` store in the port) and the voice-session stop of the oracle's
  confirmed archive; the oracle's `track_agents_overview_notification` is not synthesized (the port relies on the server's `thread/archived|deleted`
  notification, which `agents` already applies); the `Screen::requests(method)` kit helper was not added (the tests count `commands()`: the oracle's
  `ArchiveCurrentThread` event is the port's `agents::Command::Archive`).

## Outside-area edits (minimal, additive unless noted)
- core/src/threads.rs (chunk-live): `mod note; pub use note::{Note, NoteKey, Tone};`, variant `Item::Note(Note)`.
- core/src/app.rs (chunk-live): one arm `Intent::Screen(ScreenIntent::Chat(chat::Intent::Note{..})) => session_note(..)`; `session_note` in the `use session::{..}`.
- core/src/screens/chat.rs (chunk-live): `ChatNotice`, `Model.notice`, `Intent::{Notice, ClearNotice}` replaced by `Model.notes` and `Intent::Note` (the notice line is now rows of the transcript).
- src/pages/session/banners.rs (chunk-live): `NoticeLine` draws `chat.notes` as `NoteCell`s.
- src/pages/session/session_page.rs (chunk-live): `Action::ToggleSide` emits `chat::Intent::Note`; `ChatMsg::Submit` runs a draft in shell mode (a bare `!` is the help line).
- src/pages/session/cells/mod.rs (chunk-cells): `pub mod note;`.
- src/pages/session/strings.rs, transcript_pane.rs (chunk-transcript-2): `Strings::note`, the `Item::Note` arm.
- src/widgets/composer_draft.rs (chunk-composer): `ComposerDraft::recorded`.
- tests/pages/commands.rs: no one's area; registered by tests/pages/chat.rs.
- core/src/threads.rs (chunk-live), step 3: `ThreadAsk::CleanTerminals` (+ `thread()`, `wire`), the `parent_owned` refusal arm of `apply_ev` (`Submit`, `Compact`), re-export of `Stub`.
- core/src/screens/chat.rs (chunk-live), step 3: `ThreadPart.{path, parent_owned}`, `ThreadEv::Metadata` fields, `blocks_direct_input`, `ThreadIntent/Command/Ctx::CleanTerminals`, `Model.raw_output` + `Intent::RawOutput`.
- tests/kit/mod.rs, tests/kit/seed.rs (chunk-live): `Screen::commands`, `Open::held_start`.
- core/src/app.rs (chunk-live), step 4: the `NewConversation` answer also takes `agents::Reply::Started` (a fresh thread in a directory).
- core/src/screens/chat.rs (chunk-live), step 4: `ThreadPart.ephemeral`, `ThreadEv::Metadata.ephemeral`.
- core/src/threads.rs (chunk-live), step 4: re-export of `CdRefusal`; core/src/threads/note.rs: `CdRefusal`, `NoteKey::CdRefused`.
- core/src/app.rs (chunk-live), step 5: `SessionSlice.removed`, `AppCtx::Removal`, its arms in the disconnect `retain` and in `apply_reply`, one arm
  `Intent::Screen(ScreenIntent::Select(_))` (applies the select intent, then `confirmed`), `confirmed`/`removed` in the `use session::{..}`.
- core/src/threads.rs (chunk-live), step 5: `ThreadSet::forget`, re-export of `RemoveNote`; core/src/threads/note.rs: `RemoveNote`, `NoteKey::Remove`.
- core/src/screens/select.rs (chunk-popups), step 5: `Confirm`, `Menu::Confirm`, `Picked::Confirmed`, the `Pick` arm of a confirmation.
- src/sheets/select/{mod,dialog,lists,strings}.rs (chunk-popups), step 5: `open_centered`, the `Menu::Confirm` wrapper in `MenuDialog::build`, `confirmation()`, the
  confirmation words (additive).
- src/pages/session/session_page.rs, open.rs (chunk-live), step 5: `ChatState.removed` and the trigger in `did_change_dependencies`; `open_confirmation`.

## block-commands: chunk-commands, the /rename prompt view (2026-11-23, branch fleet/block-commands, tip 29abc1aa5, landed in 51abab645)
Commits: be07548c2 (`/rename` prompt view in `src/sheets/dialogs/prompt.rs`, with dialog, strings, session_page and actions; the chat page shows any model-opened prompt, so the `/export` file prompt and the `/goal` edit prompt use the same view), 569ed23c8 (pty scenario `rename-prompt`), and the merge.
Tests: five chat_commands_2 oracle tests un-pended and green. land-check on 29abc1aa5 green: 3093 tests, 5 of 5 scenarios. Binary: the `rename-prompt` scenario passes in both binaries.
Gap row: title suggestion. The host answers `Done::Title(None)` (`src/services/effects.rs`); the oracle generates the suggestion in `app/thread_title.rs` `generate_thread_title` (an ephemeral thread and a structured turn `{title}`). Need: a backend `SuggestTitle` in `services/rpc`.
Not started in this block: chunk-commands-submission, chunk-commands-export (the picker and prompt exist: verify and un-pend), chunk-commands-queue (Tab queues the raw line as `Queued::of`, drain by `dispatch_queued`, `QueueResume`), chunk-cells-mcp-inventory, chunk-cells-background-terminals, small-dialogs. Children still to create, one per NotAvailable command (`/side /btw /recap /title /statusline /logout /ide /daybreak /debug-config /subagents /tui`; the list is the NotAvailable arm of `core/src/app/session.rs`); the chunk's area widens when the next child is taken.
