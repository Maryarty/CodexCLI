# port-skeleton — parity

`codex-rs/tui-rtwk` (package `codex-tui-rtwk`, host package `codex-rtwk`):
the architecture skeleton. It is a new crate, so there is no oracle test file
to port; its tests map to the task's done-when list and to the rules of
architecture.md §4/§A6. Every number below was run: 98 tests with
`acargo nextest run --workspace` (96 in the library package, 2 in the host
package), clippy `-D warnings` over the workspace and `fmt --check` clean.

## Oracle tests
n/a: new crate; no codex-tui test belongs to the skeleton. One row stands for
how a slice fills this table (the oracle's tests of its files, one row each);
the skeleton's own tests follow it.

| oracle test | ours | ported | n/a: reason | delta |
|---|---|---|---|---|
| `codex-rs/tui/src/chatwidget/tests/composer_submission.rs::hidden_shell_paste_queued_during_turn_submits_literal_prompt` | `skeleton::queued_messages_are_sent_in_order_one_per_turn_end` (the queue only; the hidden-shell paste is S10's) | | n/a: S10 (paste, queue edit) | D2 |

| done-when / rule | ours (`tests/…::name`) | ported | n/a: reason | delta |
|---|---|---|---|---|
| the empty chat screen | `skeleton::empty_chat_screen` | | n/a: skeleton | K01 |
| one prompt, the streamed answer as plain text | `skeleton::prompt_streams_the_answer_as_plain_text` | | n/a: skeleton | K01 |
| a message typed while a turn runs is queued, shown dim above the composer, the composer clears | `skeleton::a_message_typed_while_a_turn_runs_is_queued_and_shown`, `model::a_submit_while_a_turn_runs_is_queued_and_accepted`, `model::a_second_submit_while_the_turn_starts_is_queued_not_refused` | | n/a: skeleton | D2 |
| queued messages go out in order, one per turn end | `skeleton::queued_messages_are_sent_in_order_one_per_turn_end`, `model::queued_messages_go_out_in_order_one_per_turn_end`, `model::a_turn_that_fails_to_start_still_sends_the_next_queued_message` | | n/a: skeleton | D2 |
| a queue survives a second submit | `skeleton::a_queue_survives_a_second_submit`, `model::a_queue_survives_a_second_submit_and_keeps_its_order` | | n/a: skeleton | D2 |
| a failed turn shows the server's reason | `model::a_failed_turn_keeps_the_servers_error_message`, `…_without_a_message_says_so_plainly`, `model::an_error_that_will_be_retried_is_not_shown` | | n/a: skeleton | |
| the debug menu lists the dirs and scrolls (R11) | `skeleton::debug_menu_lists_the_directories`, `skeleton::debug_menu_scrolls_to_the_cursor`, `skeleton::screen_map_registers_the_22_directories` | | n/a: skeleton | |
| the debug menu opens a slot, `q` quits, F12 opens it from chat | `skeleton::debug_menu_opens_a_slot`, `skeleton::debug_menu_q_quits`, `skeleton::f12_opens_the_debug_menu_from_chat` | | n/a: skeleton | |
| Ctrl+C quits | `skeleton::ctrl_c_quits` | | n/a: skeleton | |
| an event of an older connection is dropped (R1, epoch) | `skeleton::an_event_of_an_older_connection_is_dropped` (injects an older epoch) | | | |
| A4.2: a request opens the dialog (also under `--debug-screens`), a second one waits, dismissed once; `serverRequest/resolved` closes without a reject; resize mid-stream | `skeleton::a_server_request_opens_the_dismiss_dialog_and_is_rejected_once`, `…a_request_opens_the_dialog_under_debug_screens_too`, `…a_second_request_waits_for_the_first_dialog`, `…server_request_resolved_closes_the_dialog_without_a_reject`, `…a_resize_mid_stream_keeps_the_answer`, `model::a_resolved_server_request_closes_its_dialog_without_a_reject`, `model::a_second_request_waits_behind_the_first` | | | |
| list switch mid-stream | n/a: needs more than one thread in the UI (session-core) | | n/a: session-core | |
| the Navigator stack stands across view runs | `skeleton::the_navigator_stack_survives_a_view_run` | | | |
| state model (apply, typed tickets from commands, R9 order tolerance, R4) | `model::*` (21 tests) | | | |
| 22 screen slots render | `<dir>::<dir>_slot` (22 files) | | n/a: skeleton | K01 |
| the rtwk pin and the `[patch]` equal to codex-rs's | `pin::*` (4 tests, one diffs `[patch.crates-io]` and the ssh tungstenite patch against `codex-rs/Cargo.toml`) | | | |
| rules R1, R4, R11–R14, A6, ADR-002, ADR-010, no literal colours | `rules::*` (15 tests) | | | |
| theme at the root, tokens from `ThemeData` | `theme::*` (3 tests) | | | |
| localization: negotiation, `--locale`, fallback, rtwk strings follow, RU proof | `l10n::*` (8 tests) | | n/a: the oracle has none | |
| the command line | `cli::*` (4 tests) | | n/a: skeleton | |
| shutdown: the client comes back to `main` when the backend future is dropped | host `in_process::tests::*` (2 tests) | | | |

## Snapshots
rtwk goldens of the skeleton (`tests/snapshots/`, `UPDATE_SNAPSHOTS=1`
rewrites, compared line by line), not oracle snapshots. They changed in this
round: `chat/*.snap` lost one trailing row (the presenter no longer takes a
row below the composer), `debug/menu.snap` shows rtwk's `SelectionList`
cursor bar.

| snapshot | size | equal | delta |
|---|---|---|---|
| `chat/empty.snap` | 80x24 | no oracle (codex-tui draws its welcome and startup card) | K01 |
| `chat/answer.snap` | 80x24 | no oracle | K01 |
| `debug/menu.snap` | 80x24 | n/a: the debug menu is ours | |
| oracle `codex_tui__chatwidget__tests__compact_queues_user_messages_snapshot.snap` (queue block) | 80x24 | no, see D2 | D2 |

## Visual deltas
### D1 — chat: placeholder instead of codex-tui's chat
kind: K01
cause: other: skeleton, S10 ports the chat
oracle (`codex_tui__chatwidget__tests__compact_queues_user_messages_snapshot.snap`, rows 11-20; the welcome card and status line differ):
```
• Working (0s • esc to interrupt)

• Messages to be submitted at end of turn
  ↳ Steer submitted while /compact was running.

› Ask Codex to do anything

  GPT-5.6-Sol default · /tmp/project
  ? for shortcuts
```
ours (`tests/snapshots/chat/empty.snap`, rows 22-23):
```
ready
Ask Codex to do anything
```
proposal: accept until S10 (the chat slice replaces `ready`, the plain
`› `/`• ` rows and the footer).

### D2 — chat: the queue of messages typed while a turn runs
kind: K01 (the behaviour is codex-tui's minimal one; only the look is a stub)
cause: other: skeleton; the oracle's queue has a heading, `↳` rows, edit and
steer. Resolved in this round for behaviour: a Submit while a turn runs or
starts is queued by `apply` (one per turn end, in order) and the composer
clears only when the intent was accepted or queued. In the first round
`chat.rs` cleared the composer before `apply` had refused the message, which
lost the text: fixed (the composer clears when `ThreadModel.last_submit`
changes, i.e. on acceptance).
oracle:
```
• Messages to be submitted at end of turn
  ↳ Steer submitted while /compact was running.
```
ours:
```
(queued) two
```
proposal: accept until S10 (queue preview, edit, steer). The former kind K02
("refuses it") is gone: see delta-kinds.md.

## Scenarios (pty)
n/a until scenario-harness. By hand, `target/debug/codex-rtwk` in a 24x80 pty
(a script outside the repository, which prints only the screen text):
- empty `CODEX_HOME`: `starting…`, then `ready` and the placeholder; `hello`
  shows `› hello` and `working…`; Ctrl+C exits with status 0.
- the person's own `CODEX_HOME` (default `~/.codex`, their config and login;
  no auth file was printed, read by me or copied): starts, `ready`; with
  their config as it is, the prompt is answered by the server with `error:
  The 'gpt-6.1-sol' model is not supported when using Codex with a ChatGPT
  account.` (the `model` line of their `config.toml` names a model their
  account's models cache does not list; the reason is now shown, see the
  failed-turn tests). With `-m gpt-5.6-sol` (a model of their models cache):
  `› Reply with exactly the word: ok`, `• ok`, `ready`; Ctrl+C exits with
  status 0. So the streamed answer through the real in-process server and the
  real login works; the oracle itself was not run for comparison.

## Rules
Clippy lists (`clippy.toml`): R1 `std::thread::spawn`, `tokio::spawn`,
`Handle::spawn`, `AppCx::{spawn,channel}`, `Cx::spawn`, `spawn_tokio`,
`subscribe*`, `follow`; R11 `Instant::now`, `SystemTime::now`,
`tokio::time::Instant::now`, `chrono::{Utc,Local}::now`; R12
`fs::{read_to_string,write}`, `File::open`, `tokio::fs::*`,
`process::Command::new`, `net::{TcpStream::connect,TcpListener::bind,
UdpSocket::bind}`; R14 `use_state`, `rtwk::Obs`, controller constructors
(`TextEditingController::new`, `ScrollController::new`). Shown biting: a
temporary `process::Command::new`, `tokio::time::Instant::now` and
`TextEditingController::new` failed `clippy -D warnings` with their R-notes
(earlier `Instant::now`, `tokio::spawn`, `rtwk::Obs`).
`tests/rules.rs`: the one `#![allow(clippy::disallowed…)]` is `src/shell.rs`
(R1, allow-list test); the host package has none and spawns nothing
(`r1_the_host_neither_spawns_nor_allows`); `.unchanged(` is banned (A6; one
use at the root, the epoch drop in `ingest`, counted); the spawn paths, clock
and file system only in the shell; the transcript has no `push` (R4); ids are
newtypes (R13); no app data in widget state (R14: the scan now sees every
`State` declaration form: `pub`/`pub(crate)`/private, braces or tuple, and
`impl State for X`); controllers are made only in `src/ui/thread_ui.rs`;
no literal colours; no user-visible literal outside the string tables
(ADR-010). Shown biting: a temporary `cx.unchanged()`, the removed Navigator
key, a `Text::new("hello")` in `src/ui/view.rs` each failed their rule; the
queue test failed with the order of `send_next` reversed; the `HandBack` tests
failed with the hand-back removed.

Fix round 2: the host parses its command line inside `arg0_dispatch_or_else`
(the helper re-runs `--codex-run-as-fs-helper`, the exec helper and
`--codex-run-as-apply-patch` reach the dispatch; `host/tests/helper_dispatch.rs`
runs the built binary, 4 tests, 3 red before). Screen titles are `Strings.title`
in each `strings.rs` (no `const TITLE`); the ADR-010 grep also covers
`Text::styled`, `Span::styled`, `Text::rich`, the `format!` forms and any
`format!` literal with a word (it caught `the backend dropped N events`, now
`Failure::Lagged(n)` worded by `Shared::lagged`). Queue: a start reply that finds
its turn already ended (after a `Lagged` gap) sends the next queued message, and a
submit never overtakes an older queued one. Shown biting: the reply-side
`send_next` removed (both queue tests failed), a `Text::styled("` literal and a
word `format!` in a screen file (both ADR-010 tests failed).
ADR-002 binding conditions, numbered as architecture.md (decided-after-review
item 1) has them:
(1) `eq` widgets: every stateful widget of the skeleton is declared with
`impl_widget!(stateful eq …)`, so a view run that rebuilds the parents does
not rebuild them;
(2) the presenter sits under the Navigator and over the home: `Presenter {
child: home }` is the Navigator's home, so it is mounted once, also under
`--debug-screens`, and shows a request's dialog from `did_change_dependencies`
(`skeleton::a_request_opens_the_dialog_under_debug_screens_too`);
(3) fixed providers plus a keyed Navigator: the provider list is fixed
(session, active thread, requests, screens, handlers; the handlers with
`update_should_notify(false)`) and the Navigator carries a `GlobalKey` kept in
`CodexApp.nav_key` (`rules::adr002_the_navigator_is_keyed`; the runtime test
`the_navigator_stack_survives_a_view_run` passes with or without the key
today, since the list does not change);
(4) no same-flush decisions: the decision of a submit is `apply`'s, the view
only sends the intent (R10), and the composer clears when the model has
accepted or queued it.
ADR-003: one writer (`CodexApp::ingest` / `apply_app`, `threads::apply`),
effects as data (`AppFx`, `ThreadFx`; `AppFx.commands` is private, a thread
slot reaches the backend only through `ThreadFx::request`), the shell the only
place that starts tasks: the backend is one future run by `cx.spawn_tokio`,
its ingress a stream the shell `cx.subscribe`s to, each event stamped with the
connection epoch. Typed RPC: a `Ticket` is made only by the app's
`TicketMint`; a `Reply` is read only through `Pending::resolve`, which spends
the ticket and drops a stale one. Slots: each screen directory has a model
part, a per-thread part (`ThreadModel.slots`), per-dir `Ev`/`ThreadEv`/`Intent`/
`Command`/`Reply` variants and a `Ctx` (what an RPC was for), generated by
`screen_dirs!`.
Shutdown: quitting unmounts the app and rtwk aborts the backend task, so no
code after its loop runs; the client therefore comes back to `main` through
`HandBack` (a drop guard) and `main` awaits `client.shutdown()`.
ADR-010: see architecture.md; the shared table is `src/l10n/strings.rs`, each
screen directory has an empty `strings.rs` template.

## rtwk needs
- `run_app_with(state, view, RunOptions)` takes no `AppView`, so no
  `on_start`. `ui/run.rs` repeats its body (terminal, frame loop, overlay,
  localizations, graphics scope) over `AppHost::of_widget` to start the shell
  with `AppView::on_start`. Wanted: `run_app_with` on an `AppView`, or an
  `on_start` in `RunOptions`.
- `Tester::pump_widget(overlay::App::new(AppView))` gives the `AppView` no
  `AppHandle`, so `AppCx::quit` does nothing there; the harness uses
  `pump_app` for the quit tests. Wanted: a Tester helper that wraps in the
  overlay root and sets the handle. (`pump_app` also gives no way to set the
  overlay's theme or the localizations; the harness's `start_*` use
  `pump_widget` with `ui::run::root` and `localized`.)
- `rtwk::style` does not re-export `Attrs` (`rtwk_core::style::Attrs`) nor
  `Brightness`, hence the `rtwk-core` dependency.
- Tests need a tokio runtime entered on the UI thread (`rtwk_tokio::enter`)
  for `spawn_tokio`/`subscribe`; `Tester` has none. The harness owns one.
- The nav-proto gaps G13–G15 apply as in architecture.md §8; G15 is worked
  around by the fixed list and the keyed Navigator.
- A symlink to rtwk inside the tui-rtwk workspace does not work: cargo makes
  path dependencies under the workspace directory members, and rtwk's crates
  inherit `rust-version` from this manifest (`workspace.package.rust-version
  was not defined`; `exclude` does not help). The pin is named by absolute
  path in `[workspace.dependencies]`, one place, and `tests/pin.rs` checks
  it against `RTWK_REF`.
- `AppView` has no way to give the overlay root a theme: the theme goes on
  `overlay::App` (`ui::run::root`), which `run_app_with` does not expose.

Fix round 3 (navigation by hand): the chat shows a dim key-hints line under
the composer (`F12 screens  Ctrl+C quit`, from the string table; the chat
snapshots gained that row, ours only, no oracle row). Escape pops the route in
front through the Navigator's own back key (menu, stub routes, dialogs: it
already did, now pinned by `navigation::*`); `q` in the menu is back through
`Navigator::maybe_pop` and quits only when the menu is the home
(`--debug-screens`); the menu's hint says `esc/q back` when pushed and `q quits`
as the home. Tests: `tests/navigation.rs` (7), with the chat's transcript and
composer text checked intact after F12 and Esc. By hand in a pty on the built
binary: F12 opens the menu, a bare ESC and the kitty `CSI 27 u` both return to
the chat, `q` returns to the chat, Ctrl+C exits 0. Bites: `q` quitting again
and the hint line removed each failed their tests (the red commit).
