# usage — parity (S20)

Oracle `codex-rs/tui/src/analytics.rs` + `analytics/` (27 files), `chatwidget/usage.rs`, `reset_credits.rs`.
View `src/pages/usage/`, core `core/src/screens/usage/`. Brief `research/briefs/usage.md` §6. The page is the
app-scoped `usage::Model` in `ScreenStates` (ADR-003: one writer, `apply`; no thread state); the host runs the
loads (not yet, step 8); widget state is cursors and scroll controllers only.

## chunk-usage, state at partial 2
Branch `worktree-agent-afdce575fa199675f`. Steps 2 (core), 3 (chrome), 4 (Overview) and now 5 (plots) and
the chart reports of 6 are done; the lists (Chats, Plan), modes (`z` card dashboard, `?` help), scroll, mouse
tests, 8 (host), 9 (menu, reset credits) and 10 (binary run, generated shard) are NOT.

### Built
- **Core model** (`core/src/screens/usage/`): `mod.rs` (`Model`, `step`), `sections.rs`, `data.rs` (`Load<T>`),
  `normalize.rs`, `report_data.rs`, `tokens.rs`, `models.rs`, `profile.rs`, `date.rs`, `activity.rs`, and new:
  `plot.rs` (the plot engine: `chart(days, categories, cursor, width, height, unit, date_label) -> Plot`, a grid
  of `Mark{ch, ink}`; scales, signed bands, missing days, callout, calendar; carried from `analytics/plot*.rs`)
  and `report.rs` (`Model::focused/report/day_has_details/day_count`: the head, notes, plot, summary, header,
  breakdown rows, message, side-by-side from 140, the plot height rule; the words are the view's: `Unit`,
  `Note`, `Message`, `Label` name text, they hold none). Loads are pure `Ask` values the host will run.
- **Page** (`src/pages/usage/`): `report.rs` (the chart report: `LayoutBuilder` -> `Model::focused` ->
  `Row`/`Column` with the plot as `CustomPaint` + `CellPainter` over core's grid, table rows as
  `Row(Expanded(Text ellipsis), amount)`), `keys.rs` (the default list keymap as a `KeyBindings` table: arrows,
  `h/l`, `j/k`, ctrl+n/p/j/k/h/l, Home, End, Enter, Space, Esc), widget state in `ScreenState` (R14: a
  selected day and an open detail per report, `zoomed`; the cursors shift by 23 on a range change, as the
  oracle's `change_range`), per-report footer hints (`day`, `details`), `strings.rs` tables for every new word,
  debug entry `credits_refund` (fixture `fixtures/credits_refund.jsonl`).
- Tests: core `core/tests/usage/{plot,report,...}.rs` (13 plot tests on the 10 oracle plot snaps' rows + the
  oracle's numeric tests; 3 report tests); page `tests/pages/usage.rs`: `summary_daily_wide` (140x44) and
  **`analytics_maximized_refunds_leave_room_for_both_bands` (100x32) equal the oracle snapshots whole**, panel
  states, digit keys, the day keys, Enter/Esc detail (each shown biting once: default day 6 -> 5 reds 3).

### Gaps and needs of this step
- Secondary text (oracle `secondary_style`: a blend of the terminal's default fg and bg at 70 %, `dim` where the
  terminal has no truecolor) is drawn with the theme's dim; the selected value is the theme's bold text (oracle:
  white on a dark terminal). The kit has no palette reader, so the 10 plot snaps' palette sections are not
  compared (`tests/oracle_snap/mod.rs`, `tests/kit/compare.rs`): the inks are asserted by the marks in core.
- (closed at partial 3: the keys now go through `src/keymap`.)

### Checks at this tip
Rebased on claude/tui-rtwk `0eb53b241` (keymap merged). `acargo fmt`, clippy `--workspace --all-targets -D warnings`
exit 0, whole nextest 1773 run / 1773 pass, `shards.py --check` 0 files stale. Two reds on the way, both mine, fixed:
R14 flagged my core types `View` (now `ReportView`) and `Renderer` (now `GridPainter`) against other owners'
widget-state fields of the same name; `no_literal_colours_anywhere` flagged `Color::CYAN/MAGENTA/GREEN` in the
view: the plot's series now come from the theme's tokens (`info`, `accent`, `success`; `src/app/theme.rs` is
chunk-cells' area and could not be widened, so no new token: the terminal theme holds those three hues).

### Outside the original area (area widened on the record, additive)
- `core/tests/agents_followups.rs:243`: `usage::Ev::Invalidated{session:0}` (the Ev changed shape).
- `tests/app/navigation.rs` `usage_footer_keys_are_bound`: the live page starts at account `Loading`:
  footer `esc back` + `R retry`, R/Tab keep the page (the old test pinned the static mock page).
- `core/tests/usage.rs` (renamed into `core/tests/usage/`).

### Needs of others
- `src/pages/session/open.rs:48` (chunk-transcript) uses `usage::route()` for `/usage weekly|daily|cumulative`
  too; the page has `route_with(view)`: wire it there.
- `core/src/app.rs` + `host/src/in_process.rs` (keymap): `Command::Usage{ask}` arm and reply routing; until
  then the live page stays at Loading.
- Kit palette reader (`tests/oracle_snap/mod.rs`, `tests/kit/compare.rs`) for the 10 plot snaps.

## chunk-usage, state at partial 3 (tip is the commit named in the checkpoint)
Step A of the re-grant: modes, keys and scroll. Branch `worktree-agent-afdce575fa199675f`, on `252acefb6`.

### Built since partial 2
- **Keys through `src/keymap`** (`src/pages/usage/keys.rs`): the list context drives the page (oracle `ListAction`:
  up/down/left/right, jump top/bottom, accept, cancel, page up/down) as `KeyBindings` names of the app's `Keymap`;
  `h`/`l`/`space` stand for left/right/accept only where no action has them; the page's own keys (Tab, Shift+Tab,
  1-7, r g m R, z, q, `?`) yield to a key an action took, `?` only while the keymap leaves `?`/`shift+?` free
  (`help_shortcut_available`). Unit tests remap `cancel`->`q`, `page_down`->`?`, `move_left`->`h`.
- **Scroll** (`list.rs`): one `ScrollController` per surface (each report, the help, the dashboard) in
  `ScreenState`; PgUp/PgDn move by the viewport, Up/Down by a row on the Overview and the help (Summary
  `summary_action`), Home/End to the ends, the wheel by rtwk's own scroll view; the Overview is centred with the
  Flutter pattern (`LayoutBuilder` + `SingleChildScrollView(ConstrainedBox(min_height: viewport, Center))`).
- **Help `?`** (`help.rs`): the oracle's `help_lines` with the keymap's labels, controls listed only while
  available, wrapped by `Text(TextWrap::Word)`, the heading `Usage shortcuts` in the controls' place; Esc closes it
  before the page, `?` toggles, a digit/Tab goes to its report and closes it.
- **Footer** (`hints.rs`): the oracle's two rows (`hints.rs`) with the keymap's labels; the parts that fit the
  measured width are kept greedily (`LayoutBuilder` + `string_width`, like `page_keys::footer_fit`); the
  `pgup/pgdn scroll` part shows only while the surface in front scrolls (`ListenableBuilder` over its
  `ScrollController`, `max_scroll_extent > 0`).
- `q` closes the page (oracle: `is_done`). `z` still opens "Not implemented yet: dashboard" (the cards are not built).
- Tests (`tests/pages/usage.rs`, each shown biting once for the scroll test: PageDown made a no-op -> red):
  help open/close order, `q`, PgUp/PgDn/Home/End scroll with the footer hint appearing only when needed, the
  arrows scroll the Overview a row, mouse: tab tap, range and group control taps, wheel up/down.

### Checks at this tip
`acargo fmt`; clippy `--workspace --all-targets -D warnings` exit 0; whole nextest 946 run / 946 pass (the tree
holds the other chunks' tests only where the worktree has them); `shards.py --check` 55 shards, 0 files stale.
Rules (`binary(rules)`) green: R14 (state is indexes, flags and controllers), R15 (no segmenter in the view; only
`string_width` in a measured-width choice, as `resume/chrome.rs`).

### Run it (partial)
The real binary `codex-rtwk --debug-screens` -> `usage` -> `overview` was driven by a pty script (not part of the
tree): the Overview shows with its two footer rows (`pgup/pgdn scroll · esc back` / `tab/1–6 report · z dashboard ·
? help · R refresh`), PgDn scrolls it, `?` shows the help (heading, the lines, footer `esc back` / `? close help`),
Esc closes the help and the Overview is back, `q` returns to the debug menu. `check.py --only usage` has no scenario
(the mock model serves the Responses API only; the analytics endpoints are not mocked), so no oracle binary frame
was compared; the whole-frame equality with the oracle is the snapshot tests (`summary_daily_wide` 140x44 and the
refunds report 100x32: equal).

### Gaps and needs
- Gap (rtwk): the oracle keeps the selected rows in view while the cursor moves (`follow_selection`); rtwk has
  `Scrollable::ensure_visible` for a keyed element, but the report's selected day is cells of one `CustomPaint`, not
  an element: a long report does not follow its selection yet (Flutter: `Scrollable.ensureVisible(context)` on a
  keyed child). Not filed as a pending test yet.
- Gap (rtwk, for Chats): "Showing a-b of N chats" needs the visible range of a `ListView` (Flutter:
  `ItemPositionsListener` of `scrollable_positioned_list`); `ListController` has none. Row of the gap table to be
  added with the Chats step.
- Theme `series()` token (chunk-cells owns `theme.rs`): still the local stand-in in `Page`/`build`
  (`info`, `accent`, `success`, plain), replace when the token lands.
- Needs of others (unchanged): `core/src/app.rs` + `host/src/in_process.rs`: `Command::Usage{ask}` and the reply
  routing, until then the live page stays at Loading; `core/src/protocol/mod.rs`, `core/src/app/session.rs`,
  `core/src/commands/dispatch.rs` for reset credits (status-card step 3 is not landed: no `reset_credit`,
  `RateLimitOrigin`); `src/pages/session/open.rs` should use `usage::route_with(view)`.

### Left, in order
Card dashboard `z` (a `Grid` of titled boxes, `Updated ... UTC`, `... z to maximize`, `Report updated ... UTC` in
the help; needs `clock_format`), Chats (business table with details, consumer tasks, the `s` metric, `a`) and Plan
(periods, 12-hour clocks) with their loads in core (`AskKind`/`Ev`/`Model`), narrow Overview wrapping, host
`host/src/analytics.rs` (+ `codex-backend-client` in its own commit), usage menu + reset credits (`chat_live_6` 26,
blocked as above), English text of core into string tables (`plan_label`, `GROUP_LABELS`, `group_label`,
`tab_label`, `Load::message`, normalize errors), the generated shard (after the person's yes).
