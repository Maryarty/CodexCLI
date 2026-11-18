# Screen tests: porting an oracle test (recipe)

You port the oracle's tests (`codex-rs/tui`, tag in `docs/rtwk-port/ORACLE_REF`) as **black-box screen tests**: seed a screen, press keys or use the mouse, compare the rendered rows with the oracle's `.snap`. Your shard is `codex-rs/tui-rtwk/tests/oracle/<shard>.rs` (list: `docs/rtwk-port/test-shards.md`). Each `// ORACLE <file>:<line> <fn> ..` line is one oracle test with one stub `fn <fn>() { kit::todo(); }`. Edit only your shard file (and `tests/oracle/fixtures/<shard>/*.jsonl`). The kit is `tests/kit/`; read its `mod.rs` once.

## For each stub
1. Read the oracle test at `<file>:<line>` (in `codex-rs/tui/src/`), and its `.snap` (the `snap=` name on the manifest line; `find codex-rs/tui/src -name '*<name>.snap'`).
2. Decide, in this order:
   - **Visible?** It asserts a snapshot, screen text, or what a key/click shows: port it (step 3).
   - **Not visible**: it asserts only a value inside (events or commands sent, fields, a helper's return): `kit::na(Owner::<slice>, "asserts <what>, not rows: a state test");` and stop. Name the slice from `owner=` on the manifest line.
   - **Visible but not buildable yet** (no debug entry for the state, no way to seed it, the screen or feature does not exist): write the whole test anyway, and put `kit::pending(Owner::<slice>, "<what does not exist>");` as its first statement. Never `#[ignore]`.
3. Port:
   ```rust
   let mut s = Screen::build(Owner::DialogsPane)   // owner= of the manifest line
       .size(100, 24)                              // the oracle test's terminal size; 80x24 if it has none
       .entry("approvals", "exec_command")         // or .script(include_str!("fixtures/<shard>/x.jsonl")) or .live()
       .open();
   s.keys("down enter");                           // the oracle test's key presses, in order
   s.assert_snap("approval_modal_exec");           // the whole screen against the snapshot
   ```
   - **Seed**: what the oracle test sets up. `entry(dir, name)`: a debug-menu entry of the screen directory (`rg -n 'name: "' codex-rs/tui-rtwk/src/screens/<dir>/mod.rs` lists them; none fits: pending "no entry for <state>"). `script(jsonl)`: the live chat fed server messages, one JSON line each, copy the shape of `codex-rs/tui-rtwk/src/screens/chat/fixtures/streamed_answer.jsonl` and `tests/kit/fixtures/answered.jsonl` (texts and ids from the oracle test). `live()`: the chat as it starts.
   - **Keys** (`s.keys("..")`, spaces between keys): `KeyCode::Down` is `down`, `Enter` `enter`, `Esc` `esc`, `Tab` `tab`, `BackTab` `shift+tab`, `Char('a')` with CONTROL `ctrl+a`, with ALT `alt+a`, `Char('A')` `A`; text: `s.type_text("hi")`, a paste: `s.paste("..")`.
   - **Mouse**: `click(x, y)`, `double_click(x, y)`, `click_text("label")`, `wheel_up/wheel_down(x, y, notches)`; x is the column, y the row, from 0.
   - **Compare** (expected values come only from the oracle): `assert_snap(name)` whole screen; `assert_snap_region(name, row, col)` when the snapshot is one region of the oracle's screen (say why in a comment; the rows still must equal); `assert_snap_section(name, "section")`; for an oracle `assert!(text.contains("x"))`: `assert_contains("x")` / `assert_absent("x")` / `assert_row(i, "text")` with the oracle's literal. Run to a state: `s.wait_for("text")`, `s.wait(ms)`, `s.resize(w, h)`.
4. Run `acargo nextest run --profile oracle --no-fail-fast -E 'test(/^<shard>::/)'` (from `codex-rs/tui-rtwk`, via `~/.cache/rtwk/acargo`; `CARGO_TARGET_DIR` as your task says). A red port stays red: it is a finding for the slice porters. Then `acargo fmt` and `acargo clippy -q --all-targets -- -D warnings`.

## Never
- Never change an expected row, never trim, replace or skip rows to make a test pass. Never write an expected value of your own: no `assert!`/`assert_eq!` (a guard test rejects them), no snapshot of ours, no `UPDATE_SNAPSHOTS`.
- Never reach into core or the view: no `codex_tui_core`, `codex_tui_rtwk`, `rtwk`, no `common::` (a guard test rejects them). Do not edit `src/`, `core/`, `tests/kit/`, other shards or `tests/oracle.rs`. A kit call you miss: say so in your report, do not work around it.
- Never `#[ignore]`, never delete a stub, never rename it (names are checked against the manifest). `todo()` left in place counts as not ported.
- The only change made to our rows is key-label normalization (`shift+tab` to `⇧tab`); nothing else is, so a difference is real.

## Report
`python3 -I tests/oracle/tools/summary.py` (after the nextest run) gives pass / n/a / pending / todo / red for your shard; report that line, the red tests with their first diff row, and every pending with its reason.
