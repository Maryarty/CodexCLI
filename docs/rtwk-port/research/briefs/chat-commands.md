# Brief: chat-commands (SessionPage block "session slash commands", screen map C09)

Recon 2026-11-22, read-only (no cargo). Paths are relative to `codex-rs/tui-rtwk` (ours) and `codex-rs/tui/src`
(oracle, ORACLE_REF 29eb79e98) unless absolute. Protocol = `codex-rs/app-server-protocol/src/protocol/`.
One porter carries the whole chunk. Shards: `tests/oracle/chat_commands_{1,2,3}.rs`.

## 0. First step for the porter
1. Run the probe:
   `KIT_PROBE_PENDING=1 ~/.cache/rtwk/acargo nextest run --profile oracle --test oracle -E 'test(/chat_commands_/)' ...`,
   then `tests/oracle/tools/summary.py --list stale`.
   Stale-pending candidates (each reason was read against the tip):
   - s2:166 `inline_slash_command_is_available_from_local_recall_after_dispatch`: `/rename x` works (`ThreadIntent::Rename`, core/src/app/session.rs:112).
   - s2:359 `signed_out_usage_command_with_args_*`: seed `{"signed_in":false}` exists (core/src/screens/chat.rs:576 `SeedEv`). It still needs the record fix from step 1.
   - s1:31 and s1:44: they wait on `Screen::turns()`, which exists (tests/kit/mod.rs:154).
   - s1:630: waits on `attach_image`, which exists (kit/mod.rs:145). The steer part needs chat-live C5.
   - Reasons that say "a running task cannot be seeded" (s3:260, :274, :337, :193) are stale wording. Use `.script("running")`: `tests/oracle/fixtures/chat_commands_2/running.jsonl` seeds thread-1 + turn/started.
   - "The kit cannot set the disconnect" is stale: the agents seed `{"reconnecting":null}` already drives `detach_threads` (core/src/app.rs:1035-1050).
   - s3:207 `/stop`: the reason says "voice control". Ours maps Stop to Interrupt (core/src/app/session.rs), the oracle to a background-terminal clean. Rewrite the reason in step 3.
2. Copy only the fixture bodies that the probe shows passing. Re-word the rest with the kit step they really need (section 3, kit list).
3. Before touching shared files, read section 5 and ask the orchestrator for the order on `core/src/threads.rs` and `core/src/app.rs`.

## 1. Scope
Oracle files:
- `chatwidget/slash_dispatch.rs` 1-1420:
  - `slash_command_blocked_by_active_task` :150-170;
  - bare arms `dispatch_command_from_source` :174-630;
  - args arms :640-1140;
  - `submit_queued_slash_prompt` :1142;
  - `queued_command_drain_result` :1290-1365.
- `chatwidget/input_flow.rs:147-380`: `QueuedInputAction::{Plain,Literal,ParseSlash,RunShell}`, `maybe_send_next_queued_input`.
- `bottom_pane/chat_composer.rs:3432-3470`: stage + record slash history; clear the text except Fork|Review (`slash_command.rs:222 requires_dispatch_validation`).
- `chatwidget/interaction.rs:530-590`: rename prompt + title suggestion.
- `chatwidget/working_directory.rs:22-60`.
- `chatwidget/transcript_export.rs` (67 lines).
- `chatwidget/chatwidget.rs`: :183 PARENT_OWNED_INPUT_MESSAGE, :1551 clean_background_terminals, :1644 stub message, :1741-1753 raw toggle.
- `app/event_dispatch.rs`: :374-400 cd, :4282-4380 archive/delete.
- `app/working_directory.rs:127`, `app/transcript_export.rs:32-78`, `app/thread_title.rs:229`, `app_server_session.rs` (archive :1712, delete :1727, goal :2036-2090, clean :2210, review :2228, rollout path :3080-3140).
- `history_cell/notices.rs:334-357`: `new_info_event` / `new_error_event`.
- `assets/prompt_for_init_command.md` (path: `codex-rs/tui/assets/`).
- Shard sources: composer_submission tests (shard 1), slash_commands tests (shards 2-3), startup draft/sparkle tests (end of shard 3).

What the user sees: they type `/cmd [args]` and press Enter, or press Tab while a turn runs, which queues the command as a `↳` row that runs at turn end. Refusals and info appear as transcript cells (`■ msg` red, `• msg` dim + hint) and stay in the transcript. The line can be recalled with Up.

What the commands do:
- act on the session: new/clear/fork, compact, init, plan, goal, review;
- open views: rename prompt, export picker, archive/delete confirmation;
- show info: cd, pwd, rollout;
- toggle raw output, stop background terminals, run the stubs.

## 2. Tests (161 = 51 + 46 + 65 fns; live 7: 5 pass, 2 red; pending 116; n/a 38; source junit.xml of 04ff6c716 + shard files)
Live:
- 5 pass, all in s2: :157 `bare_slash_..._recall_after_dispatch`, :265 `interrupted_merged_...`, :335 `usage_error_..._recall`, :383 `unrecognized_..._not_added`; and s3:616 `raw_slash_..._invalid_arg`.
  - s2:335 may be a false pass: the draft stays in the composer, so Up recalls nothing new.
- 2 red:
  - s2:373 `usage_command_with_invalid_view_reports_usage_snapshot`: the oracle shows a history cell `■ Usage: ...`; ours shows a bottom notice.
  - s2:398 `unavailable_slash_command_is_available_from_local_recall`: `Outcome::Notice` never calls record (src/pages/session/actions.rs:166).

IN this chunk, pending: 46. Lines are the `#[test]` line; the `fn` is two lines below.
- s1 (12):
  - :31 `composer_submission_sends_once_and_requests_latest`;
  - :44, :57 `hidden_shell_paste_submits_literal_prompt`, `..._recalled_from_history_...`;
  - :73 `hidden_shell_paste_queued_during_turn_...` (queue Literal);
  - :118 `..._restored_after_image_rejection_...` and :133 `..._after_unavailable_model_...` (need the model catalog);
  - :165 `hidden_shell_paste_queued_before_session_...` (held start);
  - :178, :193 `parent_owned_thread_blocks_all_direct_input_entry_points`, `..._blocks_settings_shortcuts`;
  - :630 `image_submission_is_portable_for_new_turns_and_steers` (the steer half needs chat-live C5);
  - :644 `image_preparation_failure_restores_full_input_without_submitting`;
  - :659 `image_preparation_keeps_input_responsive_...` (C5).
- s2 (18):
  - :29 `service_tier_commands_...` (catalog);
  - :46 `slash_compact_eagerly_queues_follow_up_before_turn_start`;
  - :61 `queued_slash_review_with_args_restores_for_edit`;
  - :74 `queued_empty_bang_shell_reports_help_...`;
  - :90 `queued_settings_selection_applies_before_next_input` (catalog + /model popup);
  - :106 `queued_bare_rename_drains_...`;
  - :123 `queued_unknown_slash_reports_error_when_dequeued`;
  - :166 `inline_slash_command_..._recall_after_dispatch`;
  - :206 `goal_control_slash_commands_emit_goal_events` (also composer vim);
  - :222 `goal_control_slash_command_without_thread_shows_full_usage`;
  - :246 `interrupt_disambiguates_same_sized_goal_pastes` (composer paste placeholders);
  - :276, :289, :304, :321 `slash_rename_*` x4;
  - :345, :359 `signed_out_usage_command_*` x2;
  - :411 `no_op_stub_slash_command_..._recall`.
- s3 (16):
  - :41 `slash_export_opens_destination_picker`;
  - :133 `slash_keymap_invalid_args_show_usage`;
  - :193 `queued_menu_slash_keeps_agent_turn_complete_notification` (/model popup);
  - :207 `slash_stop_submits_background_terminal_cleanup`;
  - :260 `slash_clear_is_disabled_while_task_running`;
  - :274 `slash_archive_cancellation_keeps_task_running`;
  - :290 `slash_memory_maintenance_reports_stubbed_feature`;
  - :305 `slash_mcp_invalid_args_show_usage`;
  - :337 `slash_archive_confirmation_...`, :358 `slash_delete_confirmation_...`;
  - :484 `slash_app_without_thread_id_shows_starting_error`;
  - :497 `slash_pwd_and_cwd_alias_...`;
  - :527 `slash_cd_rejects_pending_input_and_unsupported_session_ownership`;
  - :554, :567 `slash_rollout_*` x2;
  - :625 `compact_queues_user_messages_snapshot` (C5 steer refusal).
In-chunk total: 53 = 7 live + 46 pending; 48 to turn on.
- About 36 stand alone.
- 10 depend on another chunk: 5 on the catalog or popups, 3 on chat-live C5, 2 on composer. Each stays pending with that chunk's name until it lands.

Other chunks (70 tests; keep pending, put the owner's name in the reason):
- chat-live C5, queue/steer/interrupt restore (11, s1): :16, :90, :103, :437, :459, :474, :507, :521, :538, :554, :615.
- composer (8): s1 :148, :218, :263, :276, :287, :326, :589; s3 :239.
- transcript (19):
  - /copy picker and copy status, s2 x13: :428, :442, :457, :477, :500, :522, :541, :558, :608, :627, :650, :693, :709;
  - s3: :10, :27, :58, :151, :179, :638.
- settings (4): s3 :70, :94, :112 (keymap capture/debug), :323 (memories: landed in 8e2667b9c, likely stale, so the settings porter probes it).
- pets (5): s3 :382, :413, :431, :447, :461.
- startup handoff (23):
  - s1 :205, :365, :383, :407, :424;
  - s3 :652 (sparkle), :667, :682, :704, :720, :743, :760, :776, :792, :809, :821, :833, :846, :863, :874, :900, :912, :930.
  - No running owner (decision a, section 7).

n/a (38): these tests assert sent ops or internal state (UserTurn, NewSession{name}, ClearUi, Fork{name}, ResumeByIdOrName, goal drafts, pet events). Keep them n/a. Their port is core tests in `core/tests/commands.rs` (step 2 adds the drain cases).

## 3. What exists in ours
- `core/src/commands.rs` (902):
  - `SlashCommand`: supports_inline_args :246, requires_dispatch_validation :299, available_during_task :325;
  - `QueueDrain` :872, `queued_command_drain_result` :882-900. The rule is ported, but no queue uses it.
- `core/src/commands/dispatch.rs` (497):
  - `Target` with opens()/dir();
  - `Notice` :104 (is_info for ModelStartup and ShellHelp);
  - `SessionIntent` (~40 variants);
  - `dispatch_command` (side/review/task_running blocks), `dispatch_bare`, `dispatch_with_args`.
- Every dispatch line already exists, including those of foreign commands:
  - /copy -> `SessionIntent::Copy` (transcript);
  - /status -> `Target::Status` (usage-status);
  - /usage -> `Target::Usage` / `UsageNeedsLogin` (usage-status);
  - /model and /permissions -> `Target::Select` + args intents (popups);
  - /theme /memories /statusline /title -> `Target::Settings` + args intents (settings).
- This chunk adds no dispatch line. It owns only the effects of its own intents (list in step 3) and the one-line forwarding arms of the foreign ones.
- `core/src/app/session.rs` (135) `session_intent`:
  - New/Clear/Fork -> `AppCtx::NewConversation`;
  - Fork -> `ThreadAsk::Fork`;
  - Compact, Rename{Some} -> `chat::ThreadIntent`;
  - `S::Stop` -> `ThreadEv::Interrupt`, which is WRONG (oracle: background-terminal clean);
  - `_ =>` -> `ChatNotice::NotAvailable`.
- `src/pages/session/actions.rs` (188): `run_line` builds `DispatchCx` with `review_mode: false` :56. It records history for NotACommand/Open/Session only; the Notice arm :166 does not record.
- `core/src/screens/chat.rs` (651):
  - `ChatNotice::{Command(Notice),NotAvailable}`;
  - `Intent::{Session,Shell,Notice,ClearNotice}`;
  - `ThreadPart` (name, cwd, model, ...; no path, no parent-owned flag);
  - `ThreadIntent::{Compact,Rename,Shell}`;
  - `metadata(&Thread)` :411, wire :542, decode :557, `SeedEv` :576.
- `core/src/threads.rs` (1703):
  - `Item` :78: `Error{message}` exists, no info item;
  - `Transcript::mint` :303, `report_error` :326;
  - `ThreadEv` :565: `Appended{turn,item}` applied :1575 with a minted key;
  - `ThreadModel.queue: Vec<UserMessage>` :692; Submit queues while busy :1426-1442; `send_next` :1654;
  - `ThreadAsk` :908 (`ForkAt{thread,cwd}` :926); wire :1000; `ThreadFx` :1118.
- `src/pages/session/session_page.rs`:
  - queue preview :605-617;
  - bottom notice line :619-632 (to remove);
  - `ChatMsg::QueueDraft` :268 (Tab) runs `run_line`, so a queued command becomes a refusal today.
- Agents: `core/src/screens/agents/update.rs:72-73` `Intent::Archive/Delete(thread)` -> `core/src/protocol/agents.rs:56-66` `Command::Archive/Delete` (wire ThreadArchive/ThreadDelete). thread/archived and thread/deleted notifications are already handled.
- Select: `core/src/screens/select.rs` (288):
  - `Menu::{Models,Permissions,List}`, `Picked::*`;
  - picks are only STORED: nothing in core/src or src reads `Picked::` (grep).
  - `List`/`Item` are in `select/list.rs`. View: `src/sheets/select/mod.rs:245` `show_dialog(MenuDialog, sheet_options)`.
- Dialogs: `core/src/screens/dialogs.rs:131 Open::{Shortcuts,Warnings,Feedback}` has no prompt. View: `src/sheets/dialogs/mod.rs:79`.
- Cells: `src/pages/session/cells/error.rs` ErrorCell ("■ {}", tokens.error()). Words: `src/pages/session/strings.rs:261-290`.

Host / protocol data -> where it enters (ADR-003: thread RPCs only through `ThreadAsk`/`ThreadFx`).

| Datum | Protocol | Enters ours |
|---|---|---|
| rollout path | `Thread.path` (v2/thread_data.rs:205) in thread/start, resume, fork responses + `thread/started` (common.rs:2053) | `chat::metadata` (chat.rs:411) adds `path` to `ThreadPart`; /rollout reads it. Fixture: notify thread/started `"path":"/tmp/codex-test-rollout.jsonl"` |
| parent-owned | `Thread.canAcceptDirectInput == Some(false)` (thread_data.rs:205; oracle agent_picker.rs:162) | `ThreadPart.parent_owned` via metadata; /compact and submit refuse with PARENT_OWNED |
| archive | `thread/archive` (common.rs:606) + `thread/archived` (:2057) | reuse agents `Intent::Archive(thread)` (agents/update.rs:72) -> `Command::Archive`; on success the app pops to AgentsPage (oracle return_to_agents_after_removal) |
| delete | `thread/delete` (:611) + `thread/deleted` (:2058) | the same, `Intent::Delete`. Oracle also deletes saved side chats (delete_thread_with_saved_side_chats): one line in core, through the same Command |
| review | `review/start` (:1179), `ReviewStartParams{thread_id,target,delivery}` (v2/review.rs:17), delivery Inline | new `ThreadAsk::Review{thread,target}` in threads.rs wire :1000. /review args = `ReviewTarget::Custom{instructions}`; the bare popup is chat-live (review_popups.rs:15) |
| goal | `thread/goal/set` :651 (`ThreadGoalSetParams` v2/thread.rs:891), `/get` :656, `/clear` :661; notif `thread/goal/updated` :2066, `/cleared` :2069 | new `ThreadAsk::{GoalSet,GoalClear}` + `ThreadEv::Goal` into `ThreadPart.goal`. The goal menu and editor views are popups (chat_popups_1); this chunk handles the args arms and the no-thread usage note |
| init prompt | none (a bundled asset) | REUSE carry `codex-rs/tui/assets/prompt_for_init_command.md` -> `core/assets/`, `include_str!` with an oracle comment, then `ThreadEv::Submit` |
| cwd change | thread/fork with the new cwd (oracle app_server_session.rs:1275 fork_thread_for_directory_change) | existing `ThreadAsk::ForkAt{thread,cwd:Some(p)}` (threads.rs:926) through the `AppCtx::NewConversation` flow. The remote rejection needs a host fact `workspace.remote: bool` (host/src/facts.rs) |
| /stop | `thread/backgroundTerminals/clean` (:792, experimental; host/src/in_process.rs:165,550 already sends experimental_api) | new `ThreadAsk::CleanTerminals{thread}` + info note "Stopping all background terminals." |
| title suggestion | no RPC: an ephemeral thread/start (`ephemeral:true`) + turn/start with output_schema + thread/unsubscribe (oracle temporary_structured_request.rs:129-145) | service `Command::SuggestThreadTitle{ticket,thread}` in src/services answering `Reply::TitleSuggested{ticket,title}` (decision b) |
| clipboard + copy sources | none (OSC 52 / markdown_copy REUSE) | transcript chunk (G9, G11). /export Clipboard reuses its clipboard service |
| export file | none: render the markdown transcript from loaded turns, write a file under cwd | `Command::ExportTranscript{thread,dest}` in host/src (file write) -> info note "Saved conversation to {path}" |
| disconnect | the app's Reconnecting/Reconnected (agents seed keys, protocol/agents.rs:280-340) | already there: app.rs:1035-1050 `detach_threads` -> `ThreadEv::ConnectionLost` |
| model catalog | `model/list` (:1185), called only at startup (core/src/screens/startup/launch.rs:120) | needs a core catalog + chat seed `{"models":[...]}`. Owner: popups. This chunk only consumes it (s1:118, :133, s2:29, :90) |

Kit additions (tests/kit, file:line):
- `tests/kit/mod.rs` after :165 (unclaimed):
  - `Screen::requests(method) -> Vec<Value>`, which wraps `fake.requests()` (src/services/backend/fake.rs:280) filtered by method. It asserts the thread/archive, delete, fork, goal and clean ops.
  - `Screen::disconnect()` / `reconnect()`, which feed the agents events `reconnecting` / `reconnected`.
  - `Screen::deliver_title(&str)`.
- `tests/kit/seed.rs` after :89 (keymap porter's area; ask for it, or have it added in their commit):
  - `Open::held_start()` calls `fake.hold_thread_start()` (fake.rs:272), plus `Screen::release_start()`.
- Keymap seed: `seed.rs:111-125 Open::keymap`, gated by `missing()` until the keymap porter lands it. The /keymap tests belong to settings; s1 queued-edit chords belong to C5.
- Model catalog seed: the chat `SeedEv::Models` (popups owns the type). Fixture line `{"screen":"chat","ev":{"models":[...]}}`.
- Image file seed: already exists (kit/mod.rs:87 `image_file`, :145 `attach_image`). The prepare-failure test removes the file itself.
- `src/services/backend/fake.rs` (unclaimed): add `hold_titles()` / `deliver_title()` beside `hold_thread_start` :272.
- Fixtures go under `tests/oracle/fixtures/chat_commands_1/` and `chat_commands_3/`:
  - rollout thread/started with `path`;
  - parent-owned thread/started with `canAcceptDirectInput:false`;
  - named thread;
  - signed-out.

## 4. Flutter -> rtwk
- Command line -> action: in Flutter a sealed `Intent` + `Actions`. Ours keeps `dispatch` -> `Outcome`/`SessionIntent` in core (exists). Effects go through `CodexApp::apply` (the bloc reducer).
- Local notes in the transcript (`■` error, `•` info + hint): in Flutter a list item appended to the history list, not a SnackBar.
  - Core: typed `Item::Note(Note)` with `Note{key: NoteKey, tone: Info|Error, hint: Option<NoteHint>}` (ADR-010: typed keys, words in strings.rs). It is appended through `ThreadEv::Appended`, which mints a key (threads.rs:1575).
  - View: `cells/note.rs` NoteCell = `rtwk::text::Text::rich` + `Span` (rtwk-widgets/src/text/widgets.rs:66, style.rs:296); dim "• " + text + dark-gray hint, or red "■ " (cell tokens of error.rs).
  - No-thread notes (/goal usage before a session) go into a session-level `chat::Model.notes: Vec<Note>`, drawn by the same NoteCell above the composer. This is app state, not thread state.
- Queue of inputs: a bloc event queue, shown in Flutter as a `Column` of rows. Ours: `ThreadModel.queue: Vec<Queued>` with `Queued::{Message(UserMessage),Literal(String),Command(String),Shell(String)}`, drained by `queued_command_drain_result` (commands.rs:882). The preview stays as `Text` rows (session_page.rs:605-617).
- Archive/delete confirmation: in Flutter `showModalBottomSheet` with two ListTiles. Ours: select `Menu::Confirm(Confirm::{Archive{running},Delete})` (select.rs) in the existing MenuDialog sheet (src/sheets/select/mod.rs:245 `show_dialog`, rtwk-overlay/src/dialog.rs:368). Words go in the select strings. The pick routes through a new app arm.
- Rename / export filename / review custom prompt: in Flutter `showDialog` + `TextField` + `TextEditingController(text: name)`.
  - Ours: `dialogs::Open::Prompt(Prompt{kind: PromptKind::{RenameThread,ExportFile,ReviewCustom}, initial, suggestion: Suggestion::{None,Loading,Ready(String)}, edited: bool})`.
  - View `src/sheets/dialogs/prompt.rs`: rtwk `TextField` (rtwk-editing/src/text_field.rs:163; `.placeholder`, `.on_submitted`, `.controller`) + `TextEditingController` (text_editing.rs:418), opened through `show_dialog` with `sheet_options` (src/sheets/mod.rs:19), as the oracle draws it in the bottom pane.
  - A suggestion that arrives sets the controller text only while `edited == false`.
  - Ref: `.claude/skills/rtwk/references/widgets/editing.md`, `overlays.md`.
- Export destination picker: select `Menu::List` with two items ("Copy to clipboard" / "Save to file"), then the prompt (ExportFile), then the host Command.
- Raw toggle: a ValueNotifier in Flutter. The raw flag lives in the transcript chunk's model; this chunk emits `transcript::Intent::ToggleRaw` + the info note (oracle chatwidget.rs:1741-1753).
- Gaps: none new. G11 (clipboard OSC 52 only) affects /export Clipboard through the transcript service. G12 (TextArea vim, paste burst) affects s2:206 and :246 through the composer chunk.

## 5. Area and conflicts
Porter area (claim):
- `core/src/commands.rs`, `core/src/commands/` (dispatch.rs);
- `core/src/app/session.rs`, `core/tests/commands.rs`;
- `core/assets/prompt_for_init_command.md` (new);
- `core/src/screens/dialogs.rs`, `src/sheets/dialogs/` (new prompt.rs), `tests/sheets/dialogs.rs`;
- `src/pages/session/cells/note.rs` (new);
- `tests/oracle/chat_commands_1.rs`, `_2.rs`, `_3.rs`, `tests/oracle/fixtures/chat_commands_1/`, `_2/`, `_3/`;
- `tests/kit/mod.rs`, `src/services/backend/fake.rs`;
- `docs/rtwk-port/parity/chat-commands.md`.

Shared files (minimal change; the orchestrator sequences them or gives them to one owner):

| File | Owner now | Change this chunk needs |
|---|---|---|
| core/src/threads.rs | unclaimed (startup-rest in review) | additive: `Item::Note`; `ThreadAsk::{Review,GoalSet,GoalClear,CleanTerminals}` + wire arms; `Queued` element type (with chat-live C5, decision c) |
| core/src/screens/chat.rs | chat-live | remove `ChatNotice` + `Intent::{Notice,ClearNotice}`; add `notes`, `ThreadPart.{path,parent_owned,goal}` in `metadata`; `ThreadEv::Goal` decode |
| src/pages/session/actions.rs | settings | Notice arm: `record(cx)` unless Unrecognized/InputTooLarge; Notice -> note intent |
| src/pages/session/session_page.rs | chat-live / transcript | delete notice line :619-632; Tab (`QueueDraft` :268) queues a command line while running instead of `run_line` |
| src/pages/session/strings.rs | transcript | new `NoteKey` words (one table block) |
| src/pages/session/transcript_pane.rs | transcript | one `Item::Note` -> NoteCell arm (~:224-290) |
| core/src/app.rs | keymap | route select `Picked` for `Menu::Confirm`/export list and dialogs `Prompt` submit to session effects; pop to Agents after archive/delete; `Command::{SuggestThreadTitle,ExportTranscript}` arms |
| core/src/screens/select.rs, select/list.rs, src/sheets/select | unclaimed (popups next) | `Menu::Confirm(Confirm)` + its rows |
| tests/kit/seed.rs | keymap | `Open::held_start()` / `Screen::release_start()` |
| host/src/facts.rs | keymap | fact `workspace.remote` |
| host/src (new export.rs) | unclaimed | transcript file write for /export |

Needs (the porter does not build these):
- the model catalog + `SeedEv::Models` (popups);
- the goal menu/editor and review popup views (popups / chat-live);
- the clipboard service + copy sources (transcript);
- collaboration-mode state for /plan (chat-status or chat-live). The arm emits only the notes "Collaboration modes are disabled." + hint / "Plan mode unavailable right now." Untested in our shards.

ADR conformance:
- ADR-003 kept: every thread RPC is a new `ThreadAsk`; notes go through `apply`; no screen copy.
- ADR-003 exception: title suggestion (decision b).
- ADR-010 kept: typed `NoteKey`.
- R15 kept: prompt and confirmation are rtwk TextField / MenuDialog.

## 6. The plan (one job, in order; tests first per step: un-pend, see red, build, green)
1. Notes + record (oracle notices.rs:334-357, chat_composer.rs:3432-3470).
   - Turns on reds s2:373 and :398, plus s2:345, :359 (signed out), s3:133 (keymap usage), s3:305 (mcp usage).
   - Core test first in core/tests/commands.rs: Notice -> Note tone.
   - Add `Item::Note` + `Note`/`NoteKey` (threads.rs), `chat::Model.notes`, NoteCell, and the transcript_pane arm.
   - Remove ChatNotice and the bottom line.
   - Record in the actions.rs Notice arm.
2. Queue of commands (input_flow.rs:147-380, slash_dispatch.rs:1142, :1290-1365).
   - Turns on s1:73, s2:46, :61, :74, :106, :123. s2:90 waits on the catalog; s3:193 on the /model popup; s3:625 on C5.
   - Core tests: drain of Message/Command/Shell, unknown slash at dequeue -> error note, empty `!` -> ShellHelp note.
   - `Queued` enum. Tab queues the raw line while running (session_page.rs:268). At turn end `send_next` (threads.rs:1654) drains: a `Command` re-enters `dispatch` via `ThreadEv::ScreenIntent`, then the next item follows per `QueueDrain`.
   - Do this AFTER chat-live C5 lands `Queued` (decision c); if C5 is not in yet, run steps 3-9 first and this step last. Edit-restore (Alt+Up) is C5's.
3. Simple session effects (slash_dispatch.rs:309-613, chatwidget.rs:1551, :1644, session_flow.rs:141,356, app_server_session.rs:3080-3140). Make `session_intent` exhaustive (no `_`): each foreign intent becomes one forwarding arm with its owner named in a comment. Arms:
   - /pwd: "Current working directory: {cwd}" (from ThreadPart.cwd).
   - /rollout: path or "Rollout path is not available yet." (metadata path).
   - /app: "Session is still starting; try /app again in a moment." / OpenDesktopThread (host fact desktop).
   - /stop: `ThreadAsk::CleanTerminals` + note (fixes the mismatch).
   - /debug-m-drop and /debug-m-update: error "Memory maintenance: {stub}".
   - /init: `ThreadEv::Submit(include_str!)`.
   - /compact: refuse PARENT_OWNED when `parent_owned`.
   - /clear: refused while running (dispatch.rs block exists; switch the test body to script("running")).
   - /raw: toggle + note.
   - Tests: s3:207, :260, :290, :484, :497, :554, :567, s1:178, :193, s2:411.
4. /cd (working_directory.rs:22-60, event_dispatch.rs:374-400, app/working_directory.rs:127).
   - Refusals: "The session must start before you can change its working directory." / "Changing directories requires an idle primary session without queued input." / the remote one.
   - Otherwise `ThreadAsk::ForkAt{cwd}` via NewConversation. Assert with `s.requests("thread/fork")`.
   - Test: s3:527 (s3:518 is n/a: a core test for ForkAt with "~").
5. Archive / delete (slash_dispatch.rs:247-308, event_dispatch.rs:4282-4380).
   - `Menu::Confirm` with titles, subtitles (running vs idle) and items from section 3. Picking Yes goes to `agents::Intent::Archive/Delete`, then pop to Agents. Error note "Failed to archive current thread: {err}" (fake `respond` error).
   - Refusals: "A thread must start before it can be archived." / "...deleted."; the side-conversation refusal.
   - Tests: s3:274, :337, :358, plus a page test for a tap on "Yes".
6. Rename prompt + title suggestion (interaction.rs:530-590, app/thread_title.rs:229).
   - Title "Rename thread" / "Name thread"; placeholder "Type a name and press Enter"; loading line "Generating a title suggestion…"; suggestion note "Suggested from this conversation"; empty -> "Thread name cannot be empty."
   - Prompt in dialogs + TextField. The suggestion keeps manual edits. `/rename` with no args opens it; with args it renames as today.
   - Tests: s2:166, :276, :289, :304, :321, plus sheet tests (tap, typing, Esc).
7. Review args + goal args (slash_dispatch.rs:991-1140, app_server_session.rs:2036-2090, :2228).
   - `/review text` -> `ThreadAsk::Review{Custom}`.
   - `/goal clear|pause|resume` -> `GoalClear` / `GoalSet{status}`; `/goal text` -> GoalSet (ConfirmIfExists opens the popups' goal confirm, named need).
   - No thread -> GOAL_USAGE note + hint.
   - Tests: s2:222; s2:206 stays pending on composer vim; s2:29 stays pending on the catalog.
8. /export (chatwidget/transcript_export.rs, app/transcript_export.rs:32-78).
   - Picker -> Clipboard (transcript service, "conversation" copy result) or File -> prompt (ExportFile) -> host write -> "Saved conversation to {path}". "No active conversation to export." without a thread.
   - Test: s3:41 + a page test.
9. Shard 1 composer-submission rest.
   - Disconnect submit: `s.disconnect()`, the oracle's refusal note.
   - Held start: `held_start` draft kept, sent on release.
   - Image prepare failure.
   - Tests: s1:31, :44, :57, :165, :644; s1:630 and :659 stay pending on C5 (steer half).
   - Re-word every remaining pending reason with its owner chunk (section 2).
10. Count: steps 1-9 turn on 38 tests; the other 10 in-chunk tests wait on catalog/popups (5), C5 (3), composer (2).
11. Pack `docs/rtwk-port/parity/chat-commands.md`, binary capture of /pwd, /rename and /archive beside the oracle, full nextest + oracle shards, `shards.py --check`.

## 7. Gaps, risks, decisions
- Risk: `/stop` today interrupts the turn; the oracle cleans background terminals. This is a user-visible difference with no gap row, fixed in step 3.
- Risk: `review_mode: false` (actions.rs:56) hides UnavailableInReview. Read the review flag from ThreadPart when chat-live models review mode; until then it is one line in the pack.
- Risk: a note scrolls with the thread transcript; the oracle inserts history above the live area. They look the same at the tail and differ only when scrolled up. One line in the pack.
- Risk: s2:335 may pass falsely (the draft stays). Its body must clear the composer before Up.
- Risk: the select `Picked` values are not routed anywhere yet (no reader of `Picked::`). Step 5 adds the first route; the popups porter will add more in the same app.rs match (one owner per wave).
- Risk: the model catalog is missing, so 5 tests stay pending until popups lands it.
- Gap rows: none new (G9, G11, G12 are touched only through other chunks).
- Decision a: 23 startup-handoff tests in our shards have no owner (startup done). Recommendation: a small separate "startup-handoff" chunk run in parallel, not this porter.
- Decision b: the title suggestion needs a temporary ephemeral thread outside ThreadFx. Recommendation: one-line ADR-003 exception ("ephemeral structured requests are a service Command, never a ThreadModel") + a debt task for a rules test that only src/services names `ephemeral`.
- Decision c: `ThreadModel.queue` changes type in both chat-live C5 and here. Recommendation: C5 lands `Queued` with Message/Literal; this chunk adds Command/Shell in step 2 after it. Otherwise one owner does both.
