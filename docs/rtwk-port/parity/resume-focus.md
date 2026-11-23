# resume-focus — parity

Bug (found by chunk-live): `codex-rtwk resume`, pick a row: the chat opened with no focus, typed keys went
nowhere until a click.

Cause: the chat was built under the picker. Its composer's `autofocus` ran while the picker's route held the
focus (a pushed route makes the route below unfocusable and remembers the primary focus to give back; the
composer had none yet), so the pop gave the focus to the home route's scope node, not to the composer.

Fix (`src/app/start_page.rs`): the resume picker stands in for the first thread, so it is the page until it pops
and the chat mounts then (Flutter: `await Navigator.push(picker)`, then `setState`; the same order as the
startup gate's `holds_child`). The composer autofocuses at mount, with whatever thread the pick opened. The
`--agents` start page is unchanged (its route has no pop notification).

Tests: `tests/pages/resume_focus.rs` (3, all red before the fix): Enter on a row, double tap on a row, Esc on
the picker, each then types `hi` and the composer shows `› hi`. The harness starts as the binary does
(`with_awaited_facts(true)` and the host's facts, gateway and launch answers); without the awaited facts the
old tree happened to pass. Outside the original area: `tests/pages.rs` (one `mod resume_focus;` line).
pages+screens+shell resume/start/navigation/agents: 210 passed. rules 66, clippy clean.

Binary: a throwaway copy of `check.py` (`--capture-resume`, mock `text-100`) that, after the picker's End +
Enter, types `zq` and requires it on the screen: red before the fix ("typed keys did not land in the
composer"), green after. land-check: all fail=0, scenarios 39/41 (known reds error-context-window,
multi-agent).

Rows that differ from the oracle: none seen; the change paints nothing new (the picker's route is opaque, so the
chat not being built under it shows nowhere).

Need / note for others: `codex-rtwk agents` -> open a thread has the same shape (`Start::Agents`, no pop
notification on its route); it was not part of this task and was not run.
