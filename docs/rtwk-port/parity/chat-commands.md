# chunk-commands — parity
Chunk `chunk-commands` of docs/rtwk-port/research/briefs/chat-commands.md (§6). Partial pack, extended at each step.
Shard counts (summary.py, oracle profile; pass / n-a / pending / todo / red):
start (claude/tui-rtwk af5799546): chat_commands_1 0/15/35/0/0 · _2 4/9/31/0/2 · _3 1/14/50/0/0.
step 1:                              chat_commands_1 0/15/35/0/0 · _2 7/9/30/0/0 · _3 3/14/48/0/0 (0 red, 0 stale).
step 3:                              chat_commands_1 1/15/34/0/0 · _2 8/9/29/0/0 · _3 10/14/41/0/0 (0 red, 0 stale).

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
