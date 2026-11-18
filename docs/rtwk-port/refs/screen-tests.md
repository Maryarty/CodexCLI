# Screen tests: porting an oracle test (recipe)

You port the oracle's tests (`codex-rs/tui`, tag in `docs/rtwk-port/ORACLE_REF`) as **black-box screen tests**: seed a screen, press keys or use the mouse, compare what is on the screen with the oracle's `.snap` or assertion. Your shard is `codex-rs/tui-rtwk/tests/oracle/<shard>.rs` (list: `docs/rtwk-port/test-shards.md`). Each `// ORACLE <file>:<line> <fn> ..` line is one oracle test with one stub `fn <fn>() { kit::todo(); }`. Edit only your shard file (and `tests/oracle/fixtures/<shard>/*.jsonl`). Read the kit's `tests/kit/mod.rs` once.

## For each stub: decide, in this order
1. Read the oracle test at `<file>:<line>` and its `.snap` (`snap=` on the manifest line; names are found by `<name>` or `__<name>`).
2. **n/a only when nothing the test asserts can be seen on a screen** (an event or command sent, a field, a returned value that is never drawn): `kit::na(Owner::<slice>, "asserts <what>, not visible");`. A helper whose output is text on the screen (footer lines, a preview, a hint, a title) is **visible**: port it as pending with the visible assertion. A test that mixes both ports the visible half and comments the rest.
   - n/a: `fn selection_returned_on_enter` asserts the `ThreadId` Enter returns, nothing is drawn. -> `kit::na`.
   - not n/a: `fn footer_lines_at_119_cells` asserts `render_footer_lines()` equals a string; that string is the footer row. -> `pending` + `s.assert_row(..)` / `assert_row_ends_with(..)`.
3. **Visible but not buildable yet** (no entry for the state, the screen or feature does not exist, no seed): write the whole test and put `kit::pending(Owner::<slice>, "<what does not exist>");` first. Never `#[ignore]`.
4. Port (`owner=` is on the manifest line):
   ```rust
   let mut s = Screen::build(Owner::DialogsPane).size(100, 24)   // the oracle test's terminal size
       .entry("approvals", "exec_command").open();                // or .script(include_str!("fixtures/<shard>/x.jsonl")) or .live()
   s.keys("down enter");                                          // the oracle's key presses
   s.assert_snap("approval_modal_exec");
   ```

## Seeds (what the oracle test sets up)
- `entry(dir, name)`: a debug entry of a screen directory (`rg -n 'name: "' codex-rs/tui-rtwk/src/screens/<dir>/mod.rs`); its data is the entry's fixture. A dialog or picker with other data than every entry has: pending "no entry with <the data>" (your JSONL in `fixtures/` is what the slice registers).
- `script(jsonl)`: the live chat fed server messages, one JSON line a step (`notify`, `request`, `screen`, `key`, `resize`; shape of `src/screens/chat/fixtures/streamed_answer.jsonl`). `live()`: the chat as it starts. `s.feed(jsonl)` applies more steps later (history prepended, a tail absorbed, a result arriving); not on an `entry` screen.
- `.early()` stops at the first frame (loading before the list); `s.wait_for("text")` runs on. `.keymap(&[("list.move_down","z")])` is pending in the kit until the port reads a keymap: keep it, the test is written.

## Keys, mouse, time
`s.keys("down ctrl+c shift+tab alt+b A")`, `type_text`, `paste`, `key_repeat("k")` (a held key), `click(x,y)`, `double_click`, `click_text("label")`, `wheel_down(x,y,n)`, `drag(x,y,x2,y2)` (or `mouse_down/mouse_drag/mouse_up`), `resize(w,h)`, `wait(ms)`, `wait_for("text")`. x is the column, y the row, from 0.

## Compare (expected values come only from the oracle)
| The oracle has | You write |
|---|---|
| a snapshot of the whole screen | `s.assert_snap("name")` |
| a snapshot of one region | `s.assert_snap_region("name", row, col)` (say why) |
| one state of a joined snapshot: `--- x ---`, `---- x ----`, `124 columns:`, `width=36`, a label over `Buffer {` | `s.assert_snap_section("name", "x")` (`"124 columns#2"` for the second one) |
| the same, as a region | `s.snap("name").frame("x").at(row, col).assert()` |
| a `Buffer {..}` dump (styles) or a `Cursor:` line | the same calls: styles and cursor are compared whenever the snapshot has them |
| `contains` / `!contains` / a row `==` | `assert_contains("x")` / `assert_absent("x")` / `assert_row(i, "text")`; `assert_has_row("text")` |
| `starts_with` / `ends_with` / fragments in order | `assert_row_starts_with(i, "x")` / `assert_row_ends_with(i, "x")` / `assert_in_order(&["a", "b"])` |
| a cell style (`fg`, `bg`, BOLD, DIM, REVERSED) | `assert_style(x, y, "fg=Cyan bold !dim")`, `assert_text_style("label", "reversed bg=Rgb(55,57,66)")` (ratatui's names) |
| the cursor | `assert_cursor(x, y)` / `assert_cursor_hidden()` |
| an OSC 8 link / the window title | `assert_link("text", "https://..")`, `assert_no_link("text")` / `assert_title("..")` |
| a column (`x == 12`) | `assert_text_at(12, row, "text")` |
| screen A equals / differs from screen B | `let a = s.shot(); .. keys ..; s.assert_same_rows(&a)` (`assert_same_rows_in(&a, 1..6)`, `assert_rows_differ(&a)`) |

Wide glyphs: the oracle writes them one char (`Hidden by multi-width symbols` note) or glyph plus space; the compare takes either. Key labels: `shift+tab` and `⇧tab` are the same; so are the shipped `delete` (macOS crossterm's name of Backspace) and the snapshots' `backspace` (the oracle's `cfg(test)` label), with the wider key column the longer name gives. The kit changes only our rows (`tests/kit/labels.rs`), never the oracle's; never normalize labels in a test file (a test outside the kit includes `labels.rs` with `#[path]`).

## Run and report
`python3 -I tests/oracle/tools/summary.py --run <shard>` (own junit file, so agents can run at once; `CARGO_TARGET_DIR` as your task says) prints pass / n/a / pending / todo / red for the shard, always. `--list red` / `--list pending` after `--run <shard>` shows the tests. A red port stays red: it is a finding. Then `acargo fmt` and `acargo clippy -q --all-targets -- -D warnings`.

## Never
- Never change an expected row, never trim, replace or skip rows to make a test pass. Never write an expected value of your own: no `assert!`/`assert_eq!` (a guard rejects them), no snapshot of ours, no `UPDATE_SNAPSHOTS`.
- Never reach into core or the view: no `codex_tui_core`, `codex_tui_rtwk`, `rtwk`, no `common::`, `oracle_snap`, `std::fs` in code (words in strings and comments are fine). Do not edit `src/`, `core/`, `tests/kit/`, other shards or `tests/oracle.rs`. A kit call you miss: say so in your report, do not work around it.
- Never `#[ignore]`, delete or rename a stub (names are checked against the manifest). `todo()` left in place counts as not ported.

## Report
The `--run` line; the red tests with their first diff row; every pending with its reason; every n/a with what is asserted; each kit call you missed.
