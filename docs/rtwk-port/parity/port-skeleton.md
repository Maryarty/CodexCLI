# port-skeleton — parity

`codex-rs/tui-rtwk` (package `codex-tui-rtwk`, host package `codex-rtwk`):
the architecture skeleton. It is a new crate, so there is no oracle test file
to port; its tests map to the task's done-when list and to the rules of
architecture.md §4/§A6. Every number below was run (58 tests in the library
workspace, `acargo nextest run`).

## Oracle tests
n/a: new crate; no codex-tui test belongs to the skeleton. What the tests
cover instead:

| done-when / rule | ours (`tests/…::name`) | ported | n/a: reason | delta |
|---|---|---|---|---|
| the empty chat screen | `screens/skeleton::empty_chat_screen` | | n/a: skeleton | K01 |
| one prompt, the streamed answer as plain text | `skeleton::prompt_streams_the_answer_as_plain_text` | | n/a: skeleton | K01 |
| the debug menu lists the (empty) dirs | `skeleton::debug_menu_lists_the_directories`, `skeleton::screen_map_registers_the_22_directories` | | n/a: skeleton | |
| the debug menu opens a slot, `q` quits | `skeleton::debug_menu_opens_a_slot`, `skeleton::debug_menu_q_quits` | | n/a: skeleton | |
| Ctrl+C quits | `skeleton::ctrl_c_quits` | | n/a: skeleton | |
| an event of an older connection is dropped (R1, epoch) | `skeleton::an_event_of_an_older_connection_is_dropped` | | | |
| a server request: dismiss dialog, rejected once (R3, ADR-002) | `skeleton::a_server_request_opens_the_dismiss_dialog_and_is_rejected_once` | | | |
| the Navigator stack stands across view runs | `skeleton::the_navigator_stack_survives_a_view_run` | | | |
| state model (apply, tickets, R9 order tolerance, R4) | `model::*` (12 tests) | | | |
| 22 screen slots render | `<dir>::<dir>_slot` (22 files) | | n/a: skeleton | K01 |
| the rtwk pin | `pin::*` (3 tests) | | | |
| rules R1, R4, R11–R14, A6, ADR-002 keyed Navigator | `rules::*` (11 tests) | | | |

## Snapshots
rtwk goldens of the skeleton (`tests/snapshots/`, `UPDATE_SNAPSHOTS=1`
rewrites, compared line by line), not oracle snapshots.

| snapshot | size | equal | delta |
|---|---|---|---|
| `chat/empty.snap` | 80x24 | no oracle (codex-tui draws its welcome and startup card) | K01 |
| `chat/answer.snap` | 80x24 | no oracle | K01 |
| `debug/menu.snap` | 80x24 | n/a: the debug menu is ours | |

## Visual deltas
### D1 — chat: placeholder instead of codex-tui's chat
kind: K01
cause: other: skeleton, S10 ports the chat
ours: `ready` status line, `Ask Codex to do anything` composer placeholder,
`› ` user rows, `• ` agent rows as plain text (no markdown, no cells).
oracle: not rendered here (S10's slice compares them).
proposal: accept until S10.

### D2 — chat: a message typed while a turn runs
kind: K02
cause: other: R10, `apply` refuses a Submit while a turn runs; the composer
clears. codex-tui queues the message.
proposal: follow-up task S10 (composer queue).

## Scenarios (pty)
n/a until scenario-harness. By hand: `target/debug/codex-rtwk` in a 24x80
pty with an empty `CODEX_HOME`: starts, shows `starting…` then `ready` with
the placeholder, a typed `hello` shows `› hello` and `working…`, Ctrl+C exits
with status 0. The streamed answer against a mocked model was NOT run through
the real in-process server (no model mock in this task); it is run through the
scripted backend (`shell/fake.rs`) in `skeleton::prompt_streams_…`.

## Rules
Clippy lists (`clippy.toml`): `std::thread::spawn`, `tokio::spawn`,
`Handle::spawn`, `AppCx::{spawn,channel}`, `Cx::spawn`, `spawn_tokio`,
`subscribe*`, `follow` (R1); `Instant::now`, `SystemTime::now`,
`fs::{read_to_string,write}` (R11/R12); `use_state` (R14); `rtwk::Obs`.
Shown biting: a temporary `Instant::now`, `tokio::spawn` and `rtwk::Obs` in
`src/ui` failed `clippy -D warnings` with their R-notes.
`tests/rules.rs`: the one `#![allow(clippy::disallowed…)]` is `src/shell.rs`
(R1, allow-list test); the host package has none and spawns nothing
(`r1_the_host_neither_spawns_nor_allows`); `.unchanged(` is banned
(A6; one use at the root, the epoch drop in `ingest`, counted); the spawn
paths, clock and file system only in the shell; the transcript has no `push`
(R4); ids are newtypes (R13); no app data in widget state (R14). Shown biting:
a temporary `cx.unchanged()` in `chat.rs` and the removed Navigator key each
failed their rule.
ADR-002 conditions: (1) the fixed provider list (session, active thread,
requests, screens, handlers); (2) handlers provided with
`update_should_notify(false)`, readers read the current handlers when they
emit; (3) the Navigator carries a `GlobalKey` kept in `CodexApp`
(`rules::adr002_the_navigator_is_keyed`; the runtime test passes with or
without the key today, since the list does not change); (4) the decision of a
submit is `apply`'s, the view only sends the intent; the presenter sits under
the Navigator and shows dialogs from `did_change_dependencies`.
ADR-003: one writer (`CodexApp::ingest` / `apply_app`), effects as data
(`AppFx`, `ThreadFx`), the shell the only place that starts tasks: the backend
is one future run by `cx.spawn_tokio`, its ingress a stream the shell
`cx.subscribe`s to, each event stamped with the connection epoch.

## rtwk needs
- `run_app_with(state, view, RunOptions)` takes no `AppView`, so no
  `on_start`. `ui/run.rs` repeats its body (terminal, frame loop, overlay,
  localizations, graphics scope) over `AppHost::of_widget` to start the shell
  with `AppView::on_start`. Wanted: `run_app_with` on an `AppView`, or an
  `on_start` in `RunOptions`.
- `Tester::pump_widget(overlay::App::new(AppView))` gives the `AppView` no
  `AppHandle`, so `AppCx::quit` does nothing there; the harness uses
  `pump_app` for the quit tests. Wanted: a Tester helper that wraps in the
  overlay root and sets the handle.
- `rtwk::style` does not re-export `Attrs` (`rtwk_core::style::Attrs`), hence
  the `rtwk-core` dependency.
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
