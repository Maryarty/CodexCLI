# rtwk-repin-5: tui-rtwk on rtwk main c953f062

Branch `fleet/rtwk-repin-5` (from `claude/tui-rtwk` 8cf058010), worktree
`.claude/worktrees/rtwk-repin-5`. The person's decision (2026-11-24): the tip, not a
tag. The pin is `~/.cache/codexcli/rtwk-pin-c953`, rtwk main `c953f062`, 346 commits after
`5583c142`. What it brings that the port waits for: `KeyPress::layout_char` and the layout
keys (w2-composer item 9, the Cyrillic half of #35599) and ADR-029 (selection takes twk's
shape). New crates on it: `rtwk-input`, `rtwk-mermaid`, `rtwk-driver`.

## What changed

1. Pin: `Cargo.toml` paths, `docs/rtwk-port/RTWK_REF`, `README.md` (line 22),
   `research/briefs/transcript.md`, the rtwk skill regenerated (`generate.py --check`
   green, 40 files), `Cargo.lock`. `tests/rules/pin.rs` (the pin test; the task text named
   `tests/pin.rs`, which does not exist) passes.
2. Compile on the new API: one error. `KeyCode` gained `Modifier`, `Lock` and `Media`
   (kitty reports a modifier pressed alone), so `keymap/press.rs::stroke_of` returns `None`
   for them. No layout workaround was written for the new API (R15). The keymap
   inspector's raw names for these keys fall back to `{:?}`; the app never pushes
   `ALL_KEYS_AS_ESCAPES`, so it is unreachable.
3. Selection on ADR-029 (the pin still has ADR-028's `ItemTexts`, `selection_text`,
   `find_in_items`, `select(target, ..)` until rtwk's `sel-028-retire`; the port no longer
   builds on them):
   - Gutters (the prompt mark, bullets, tool-row marks, the cut mark of `dots`, the
     transcript activity glyph) are wrapped in `rtwk::gestures::ExcludeSelection` in
     `cells/{warning,notice,viewed_image,tool_rows,disclosure,user,agent,reasoning,delegated}.rs`,
     `transcript_activity.rs`, `widgets/dots.rs`. `GUTTERS_SELECTED` is deleted.
   - `cells/text.rs`: `Texts::as_mounted()` is what a `SelectionArea` over the mounted cell
     selects (`joined(false)`); the docs describe the `ListView::item_text` model.
   - `transcript/item_text.rs`: `item_text(item, env)` lost its `bullet` parameter; the
     Agent arm is `markdown_text::text(&parse_markdown(text))`.
   - `cells/note.rs`: a note's `texts()` is one text with its mark in front (Info
     `bullet`, Done `check`); new test `a_note_is_one_text_with_its_mark_in_front` and a
     `Item::Note` arm in `text_contract`.
   - `tests/pages/rtwk_g9.rs` rewritten on `ListView::item_text` + `selection_of` +
     `SelectionAreaController::select_between` / `select_all`: a drag from the first entry
     copies the source of entries scrolled out; a selection keeps its entries when older
     history is prepended (re-selected by `SelectionPoint`s, not keys).
   - `tests/oracle/pager.rs`: four pending reasons reworded (find, select and copy stay
     pending: w2-transcript run 3 wires them).
4. Docs: `rtwk-gaps.md` (tally at c953f062, G9 amended, G61 to G64 and G73 re-checked,
   new row `G-new-sel-lines`), `parity/cells.md` (as_mounted, gutters), the transcript
   brief ("Selection + copy" superseded by ADR-029).

## What the pin lost or changed for us

- ADR-029: an item's text is one line. No multi-line text per item, no keys, no
  `ListView::point_of` on the pin. A transcript cell is many wrapped rows, so a cell
  cannot be one `item_text` the way the oracle copies it. Row `G-new-sel-lines` (open):
  wanted multi-line item text or the lazy region's text source, plus a selection kept over
  a prepend. w2-transcript run 3 decides how the transcript wires it.
- "The default theme is the terminal's colors": the app root theme is already
  `ThemeData::terminal(Brightness::Dark)`, so it is unaffected.
- `MarkdownView` options on the pin: theme, hyperlinks, link_destinations, code_wrap,
  tab_width, fence_builders, on_link_tap. No soft-break, list-indent or spacing option.

## Gap rows (`docs/rtwk-port/rtwk-gaps.md`, committed here)

| row | status after c953f062 |
|---|---|
| G9 | amended, not reopened: ADR-029 shape, probe `rtwk_g9` on it; the transcript's need is `G-new-sel-lines` |
| G-new-sel-lines | new, open (one line per item, no keys, no `point_of`) |
| G61 math | open: `FenceBuilders` draws a ```math fence; the oracle's `$..$` / `\[..\]` in running text no builder sees |
| G62 mermaid | narrowed: `FenceBuilders` keyed `mermaid` is the door; `rtwk-mermaid` parses and models flowcharts, no drawing |
| G63 soft break | open, no option |
| G64 list indent | open, no option |
| G73 | open, unchanged |
| layout keys | `KeyPress::layout_char` (1cf7a27c) is on the pin; no row named it; w2-composer items 9-10 and #35599 are unblocked |

No gap row is closed by the 346 commits.

## Oracle counts (`--profile oracle`, `summary.py`)

| | pass | n/a | pending | todo | red | stale |
|---|---|---|---|---|---|---|
| after | 1215 | 301 | 1098 | 0 | 0 | 0 |

(The tip moved a lot since repin-4's table: the baseline is the land-check of `8cf058010`;
no test changed state because of the pin.) The probe run (`KIT_PROBE_PENDING=1`) lists
stale pending tests that are unrelated to the pin.

## Checks

- One justified run each (`PORTER_RUN_OK=1`: the behaviour of a 346-commit pin cannot be
  judged without a run): default profile, 1719 tests, 1 failure
  `kit_proof::opening_a_script_screen_gives_the_same_rows_every_time`: a clock-minute
  rollover flake (10:37 against 10:38), unrelated; oracle profile as above; the skill
  snippets (`compile-snippets.sh`) 7/7.
- `acargo clippy --workspace --all-targets -- -D warnings`: clean. `--test rules`
  (includes `pin.rs`): green. `acargo fmt` and `rustfmt --edition 2024 tests/{...}/*.rs`:
  clean.
- No pty run (refused for porters): the orchestrator's batch names the scenario below.

## For the lander / orchestrator

- Edits outside the listed area: `rtwk-gaps.md`, `parity/cells.md`, and the gutter wraps
  across `cells/*`, `transcript_activity.rs`, `widgets/dots.rs` (one line each). Another
  slice touching the same cells (w2-status, kit-seeds) may conflict textually.
- `item_text()` has no `bullet` parameter any more (w2-transcript run 3 calls it).
- Visible change: none expected in the rows; a drag over a message now leaves out the
  gutters through `ExcludeSelection` instead of the old `GUTTERS_SELECTED` text trick.
- key: `pages::a_drag_from_the_first_entry_copies_the_source_of_entries_scrolled_out`,
  `pages::a_selection_keeps_its_entries_when_older_history_is_prepended`,
  `pages::a_note_is_one_text_with_its_mark_in_front`, `pages::text_contract`,
  `rules::pin`.
- pty: startup, one chat turn, then a mouse drag over the transcript and a copy: the
  copied text has no prompt mark or bullets.
