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
- The keys are a const table (the default keymap), not the keymap view API of the keymap owner: wire when the
  rebase brings `src/keymap` (`tui.keymap` remaps).

### Checks at this tip
`acargo fmt`, clippy `--workspace --all-targets -D warnings` exit 0, whole nextest 857 run / 856 pass; the
one red is the known flake `shell_agents::typing_a_word_asks_the_server_once` (known-red.txt),
`shards.py --check` 0 files stale. R14 (`r14_widget_state_fields_hold_no_screen_model_types`) was red
because two of my core types were named `Cell` and `What` (the scan flags `Rc<Cell<bool>>` of other
owners and a doc comment of agents); renamed to `Square` and `AskKind`, green now.

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

### Left, in order
Lists (Chats, Plan, the `s` metric, zero-credit groups; tasks/chats/plan loads are not in the model yet),
modes (`z` card dashboard with `Updated ... UTC`, `?` help, focus), PgUp/PgDn scroll of a long report
(`SingleChildScrollView` + `ScrollController`, the `PgUp/PgDn scroll` hint) and the wheel, mouse tests (tabs,
controls, a tap on a day of the plot, wheel), a page test of the range change keeping the selected date, narrow
Overview wrapping and footer fit, host service `host/src/analytics.rs`, usage menu + reset credits
(`chat_live_6` 26), English text of core into string tables (`plan_label`, `GROUP_LABELS`, `group_label`,
`tab_label`, `Load::message`, normalize errors), the generated shard (after the person's yes), binary run
(`check.py --only usage`).
