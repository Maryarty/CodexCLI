# chunk-commands — parity
Chunk `chunk-commands` of docs/rtwk-port/research/briefs/chat-commands.md (§6). Partial pack, extended at each step.
Shard counts (summary.py, oracle profile; pass / n-a / pending / todo / red):
start (claude/tui-rtwk af5799546): chat_commands_1 0/15/35/0/0 · _2 4/9/31/0/2 · _3 1/14/50/0/0.
step 1:                              chat_commands_1 0/15/35/0/0 · _2 7/9/30/0/0 · _3 3/14/48/0/0 (0 red, 0 stale).

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
