# onboarding-focus — parity

Bug (third case of resume-focus / agents-focus, see `agents-focus.md` Need): the folder-trust prompt cancelled at launch
replaces the startup page with the command center (oracle `lib.rs`: `consent.should_exit` ->
`SessionSelection::AgentsOverview`); a task opened from it left the chat with no focus, typed keys went nowhere until a
click. Also: with `agents`/`resume` on the command line the gate stayed closed on that branch (`left` was not told), so
the screen went blank when the center popped.

Reproduced first: `tests/pages/onboarding_focus.rs` (3: plain launch, `agents`, `resume`; each shows the trust prompt,
cancels it with Esc, opens the task with Enter in the center, types `hi` and expects `› hi`). Red before the fix: the
plain launch showed the chat without `› hi`; `agents` blank. Harness as the binary: `with_awaited_facts(true)`, the
host's facts with the trust ask, gateway and server answers. Green after.

Cause: the chat was built under the startup page (the gate opened as it pushed the page), so its composer's
`autofocus` ran while a route held the focus; `push_replacement(start_route())` moved on with the chat already mounted
and nothing mounted it afresh after the center popped.

Fix: the chat mounts once the startup page is gone, always (Flutter: `await Navigator.push(page)`, then `setState`).
`StartupGate` no longer opens as it pushes the page: it pushes `route_then(step, Left { page, center })` and opens on
`Popped`, told by the page when it pops (`left.page`) or, with the trust prompt cancelled, by the center that replaced
it (`left.center`, `start_route_then`: a `PopScope.onPopInvokedWithResult`). `holds_child` is gone (one field of
`view.rs`, outside the first area): holding is the rule, not the start pages' case. `StartPage` reads `overview(model)` as
it mounts: the center was the start page already (whatever the command line said), so it pushes nothing, tells core
`StartPageLeft` and shows the chat.

Tests: onboarding_focus 3 (red -> green); pages+screens+shell: 1011 passed; rules 78; clippy clean.

Binary (`target/debug/codex-rtwk`, throwaway driver in a pty: the oracle saves a session with `exec`, the folder is
made a git repo with no trust entry, then Esc on the trust prompt, Enter on the task, type `zq`): plain launch and
`resume --no-daemon`: trust prompt, center, chat, `zq` echoed (green). `agents --no-daemon` asks no trust prompt in
either binary (the shared overview does not open a workspace), so that branch is covered by the page test only. The
oracle's Esc on the prompt opens the center too; opening a task there asks the folder's trust again ("Back to Agent
Command Center"), which rtwk does not (that is the oracle's `existing_task` trust, a different branch from this one).

Rows that differ from the oracle: none seen in what this changes; the second trust question of the oracle after
opening a task from the cancelled-trust center is not built here (not part of this task).
