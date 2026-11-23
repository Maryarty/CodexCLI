# agents-focus — parity

Bug (the shape resume-focus found, see `resume-focus.md`): `codex-rtwk agents`, open a task: the chat opened with no
focus, typed keys went nowhere until a click.

Reproduced first: `tests/pages/agents_focus.rs` (2: Enter on a task, double tap on a task; each then types `hi` and
expects the composer to show `› hi`), red before the fix (the harness starts as the binary does: `with_start_agents`,
`with_awaited_facts(true)`, the host's facts, gateway and launch answers).

Cause: the same as resume-focus. The chat was built under the command center's route; its composer's `autofocus` ran
while that route held the focus, so the pop gave the focus to the home route's scope node.

Fix: `Start::Agents` goes the way `Start::Resume` went. The center is the page until it pops and the chat mounts then
(Flutter: `await Navigator.push(center)`, then `setState`). `pages::agents::start_route_then(left)` wraps the center in
a `PopScope` whose `onPopInvokedWithResult` tells `StartPage`; `StartPageState.shown` starts false for both start
pages. `start_route()` (no notification) stays for onboarding's overview branch (see the Need).
`StartPageLeft` also reaches core on this path: `start_fresh_thread` does nothing while a thread is active, opening or
coming, so a task opened from the center keeps its own thread.

Tests: agents_focus 2 (red -> green), resume_focus 3; pages+screens+shell agents/start/navigation/resume/startup:
215 passed; rules 78, clippy clean. Outside the original area: `tests/pages.rs` (one `mod agents_focus;` line).

Rows that differ from the oracle: none seen; the center's route is opaque, so the chat not being built under it shows
nowhere.

Need / note for others: `src/pages/startup/onboarding/mod.rs:341` (the trust prompt cancelled at launch replaces the
startup page with `agents::start_route()`): without a start page the chat is built under that page and nothing mounts
it afresh after the center pops (the composer's focus has the same shape); with `agents`/`resume` on the command line
`holds_child` keeps the gate closed and `left` is not told on that branch (`cx.emit(left, ())` is in the other one).
Not part of this task and not run.
