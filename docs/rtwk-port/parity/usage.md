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

## chunk-usage, state at partial 4 (step 1 of the second re-grant: the card dashboard `z`)
Branch `worktree-agent-afdce575fa199675f`, step 1 on `6c8adf127` (rebased onto `af5799546`).

### Built
- **Dashboard `z`** (`src/pages/usage/cards.rs`, `core/src/screens/usage/dashboard.rs`): six cards of titled
  rounded boxes (`BoxWidget` + `Stack`/`Positioned` for the two-style title `▸`/section name, cut by the text's
  own ellipsis), two columns from 104 cells, one below; heights from core (`dashboard::Geometry`: 16-24 rows,
  the plot grows with the tallest card, never with a card's content), the card's lead from `Model::card_lead`
  (range, model, grouping, the long range on the cards of its group), `OverflowBox` in a `SizedBox` cuts a taller
  card at its rows. One `ListView` + `ListController` (the dashboard's `ScrollController`); the selected card is
  brought into view only when it changes (`Reveal`, a small stateful widget: `did_update_widget`), as the oracle
  follows the selection on a section change and not on the wheel/page keys.
- Tap selects a card, double tap opens its report (`Msg::Open`), Esc closes the page from the dashboard, Enter
  focuses the report keeping section, day and grouping; the `Dashboard` control returns to the report (ADR-011).
- Help ends with `Report updated {time} UTC`; cards carry `Updated {time} UTC`, the controls' last row
  `Updated · {time} UTC`; the account's clock preference (`hour24`) settles with the account in core
  (`AccountReply`, serde default false) and drives every time here.
- Fixtures through the core `apply` (`fixtures/dashboard.jsonl`, `dashboard_denied.jsonl`, `updated.jsonl`; the
  series and weights are the oracle's `test_fixtures.rs` formulas); debug entries `dashboard`, `dashboard_denied`,
  `updated`.
- Tests: `dashboard_cards_align_in_two_columns_at_144` and `dashboard_cards_stack_in_one_column_at_72` compare
  the two halves of the oracle snapshot `dashboard_cards_align_and_keep_stable_summary_heights` row for row
  (equal; the narrow one after the Plugins report is refused, the day moved with Left, then Enter focuses it
  again with the day kept); Esc; follow-the-card at 80x24; tap and double tap; the report time in help, card and
  controls; 9 core tests (`tests/usage/dashboard.rs`: geometry, lead, plot rows; `hour24` settling in `state.rs`).
  Shown biting: double tap mapped to select -> the tap test red; `Reveal` never scrolling -> the follow test red.

### Checks at this tip
`acargo fmt`; clippy `--workspace --all-targets -D warnings` exit 0; whole nextest 2043 run, 2042 pass, 1 fail:
`chat::chat_live::a_tap_on_a_row_chooses_it` (not this area; passes alone: a load flake of the whole run);
`shards.py --check` 55 shards, 0 stale. Rules green (R14: the controllers are made in `Default for ScreenState`
by a loop, not a closure; R15).

### Chats and Plan cards
Placeholders until step 2: the cards show the section's load message ("No history has been reported." in the
oracle's fixture, which has no chats), so the snapshot equality holds; the cards of real Chats/Plan data come
with their loads.

## chunk-usage, step 2a: the Plan report (card and focused)
Core: `core/src/screens/usage/plan.rs` (`Data`/`Report::parse` of the wire's history, `Pos`/`Step` for the cursors,
`Model::plan_view` -> `View { tables, notes }` of `Line`s, `side_by_side`/`table_width`), `Model.plan: Load<Rc<Report>>`,
`AskKind::Plan`, `Ev::Plan`, `Seed.plan`; asked only for a consumer account with the feature on; refresh/close drop it.
Page: `src/pages/usage/plan.rs` (a table is a `Column` of rows, each wrapped by word at its width; a period's row puts
its share at the right when the pair fits one row and wraps as one text when not, as the oracle's `columns` + wrap;
the row under the cursor of the table in front is a `BoxWidget` filled with the selection style; two tables in a
`Row` of two fixed-width `SizedBox`es from 100 cells, one under the other below), `strings.rs` (plan words),
controls row `←/→ 5-hour / Weekly` (click: the other table), `list.rs` (Plan keys), `hints.rs` (`enter details` where a
period is under the cursor), `cards.rs` (the Plan card: first period of each table + notes).
Mouse (ADR-011, beyond the oracle's control hit): a tap on a period picks it in its table, a second opens its details,
a third closes them (handlers made by `retap`, as many as the history holds).
Fixtures (core `apply`): `plan`, `plan_twelve_hour`, `plan_unavailable`, `plan_empty`, `plan_zero`, `plan_future`,
`plan_many`; debug entries of the same names.
Tests: oracle snapshots equal row for row: `period_navigation_...` both screens (120x30, 58x30), `unavailable_empty_and_
zero_history_are_distinct` (four states at a body of 100), `plan_clocks_twelve_hour`; `plan_escape_collapses_details_
before_closing`, `moving_period_selection_collapses_its_previous_details`, control click (only on painted cells), period
click, `enter details` hint, the Plan card; core: window split/sort, invalid history refused without wire text, plan
asked only where shown, settle by stamp, drop on refresh/close, the keys (`Pos::step`), Esc order, new snapshot resets,
click picks, card vs report rows, notes, side-by-side width. Shown biting: Esc always closing -> `plan_escape_...` red.
Gap filed: **G-new-usage-1** (the Plan report and the chart reports do not follow the cursor: `Scrollable.ensureVisible`
of a row is not reachable from a State), `the_plan_report_follows_the_cursor_row` pending; the oracle's
`resize_keeps_visible_weekly_selection_and_respects_manual_scroll` rests on it and is not ported.

## chunk-usage, step 2b: the Chats report (card and focused)
Core: `core/src/screens/usage/chats.rs` (the listings as the host hands them: `ChatList`/`ChatRow`/`Estimate` for a
business account, `TaskList`/`TaskRow`/`Amounts` for a consumer one; `balance_text` (the oracle's `task_panel::amount`
balance rounding), `Metric` and the sort `compare`; `ChatPos`/`ChatStep` for the cursor and the open row;
`Model::chat_view`, `task_view` (card or report), `chat_card` as `ChatLine`/`TaskLine` values; the constants of the
oracle's widths), `Model.chats`/`tasks: Load<Rc<..>>`, `AskKind::Chats`/`Tasks`, `Ev::Chats`/`Tasks`, `Seed.chats`/`tasks`,
`Intent::TaskMetric` (cycles the metrics offered). Asked when `Section::Chats` is shown, by account kind; refresh/close drop.
Page: `src/pages/usage/chats.rs` (a row is a `Row` of an `Expanded` title cut with an ellipsis and its amounts in
fixed-width right-aligned `SizedBox` cells; the row under the cursor a `BoxWidget` filled to the report's width; the
dollar column stacks under the title below 70 cells, the metrics all show when `widest + gaps + 24` fits, the breakdown
is a table from 70 cells: each a decision made from the measured width through `segmenter_of(cx)`, no string cutting;
rules by `Divider`), `strings.rs` (Chats/Tasks words), `list.rs` (`ChatStep` keys, PgUp/PgDn scroll), `hints.rs`
(`enter details` through `chat_has_details`), `cards.rs` (business card: first five rows; consumer card: `task_view`
without details), controls row `s sort: <metric>` (click = `s`), keys `a` (zero-credit groups, when a row is open)
and `s`. Mouse (ADR-011): a tap on a row picks it, a second opens it, a third closes it.
Fixtures (core `apply`): `chats`, `chats_long`, `chats_usd`, `chats_many`, `tasks`; debug entries of the same names.
Tests: oracle snapshots equal row for row: `chats_breakdown_ranks_nonzero_groups_and_reflows` (110 and 58 wide, then
`a` shown), `chats_long_title_keeps_detail_indent_and_counts_missing_estimates`, `consumer_chat_all_metrics`; the
dollar column wide and stacked, the details hint following the row, the sort control click, both cards, row taps;
core: the balance table, missing rows last and metrics offered, sort order, `ChatPos` steps and taps, groups ranked
and zero ones hidden, the dollar column, the card's five rows, asks by account kind and stale stamps dropped.
Gap filed: **G-new-usage-2** (the oracle's `Showing a-b of N chats` is the window the viewport shows; rtwk cannot say
which rows a scrolled list shows), `the_chats_report_says_which_rows_the_window_shows` pending. The report shows every
row under one scroll and says `Showing 1-N of N`; **G-new-usage-1** also keeps the cursor row from being followed.
Needs (core-wiring/keymap): none new; the host side of `AskKind::Chats`/`Tasks` (`thread/list` roots + estimates,
`task usage`) is step 3.

## chunk-usage, step 3: the host side (`host/src/analytics*`)
Branch `worktree-agent-afdce575fa199675f`. The asks of core run as `Command::Usage{ask}` (the variant exists in
`core/src/app.rs` since core-wiring; `usage::emit` pushes it now, nothing is stubbed) and are answered with
`AppEv::Screen(ScreenEv::Usage(ev))` under the ask's session and stamp.

### Built
- `host/src/analytics.rs`: `Store` (the open session of the page: the number core gave it and its `Live`; an ask of a
  newer session opens it, an ask of a replaced one is dropped), `run(startup, ask, ids, send)` (one arm in
  `in_process.rs::work`, `Startup.analytics`), the timeouts of the oracle's `Load::start` (30 s; chats 90 s; tasks
  120 s), `Ev::Invalidated` instead of the answer when an identity check failed, the account reply (plan, label,
  `Utc` day of the session, `AnalyticsPlanHistory`, the model names from `model/list`, credit groupings, the clock
  preference).
- `analytics/client.rs`: the oracle's `Live` (session from the config, identity checks, payload cache keyed by route and
  range, plan routing per account kind, token models and attribution returned in the `ReportReply`); `convert.rs`: the
  backend's payloads as core's values (`AnalyticsResponse`, `PlanLimitHistory`, `AccountProfile`); `chats.rs`: the
  `thread/list` roots, the business estimates (batches, 503 splitting, 25 s budget), the consumer task usage with the
  descendant proof; `clock.rs`: the 12/24-hour preference (macOS `NSDateFormatter`, glibc `nl_langinfo_l`, Windows
  short time) read once.
- Host Cargo: `codex-backend-client` (own commit, `Cargo.lock`), and the platform crates of the clock detection.
- Tests: `host/src/analytics/tests.rs` (roots: cutoff, ephemeral and child filter, cursors, repeated cursor, refused
  listing; descendants: both archived passes, unsupported filter, incomplete chain, cycle; the plan history and the
  profile decoding; the session `Store`), `core/tests/usage/state.rs` (the asks leave as `Command::Usage`, the account
  answer through the app starts the reports).

### Outside the area (one line, to keep the build green)
- `src/services/clock.rs` + `pub mod clock;` in `src/services/mod.rs`: R11 allows the clock only in services (the host
  crate cannot call `Utc::now`/`Instant::now`), so the wall clock (`unix_now`: the session's day, the 30-day cutoff of
  the chat listing) and the 25 s budgets (`deadline_in`, `passed`) are read there. host-services may move it.

### Needs
- The scripted fake (`src/services/backend/fake.rs`) records `Command::Usage` and answers none: a test injects the
  `AppEv`; the host's loads that need the backend's account session (`AnalyticsSession::from_config`, the identity checks)
  run only against a real sign-in, so the binary scenario shows the sign-in error text of a machine with none.
- The English of the load errors (`Request timed out...`, the report errors, `Untitled chat`) is in the host as in
  the oracle's data layer; step 5 may give core a typed error if the person wants them in the string tables.

## chunk-usage, step 4: the usage menu and reset credits (not built, blocked)
Oracle rows: `chat_live_6.rs:12-394` (the `/usage` menu of the rate-limit prompt, the reset-credit actions). They stay
`kit::pending` in `tests/oracle/chat_live_6.rs` (owner chat-live).
### Needs
- Core data: the reset-credit state and the rate-limit origin (`RateLimitOrigin`) exist nowhere in core or on
  `claude/tui-rtwk`; they come with the account/rate-limit snapshot of core-wiring (`core/src/protocol/`,
  `core/src/app.rs`, `app/session.rs`, `commands/dispatch.rs`) and status-card step 3 (the `/status` card reads the same
  snapshot). The menu itself is a chat popup (chunk-popups) opening `UsagePage` or the credit reset ask; nothing in
  `src/pages/usage/` is missing for it once the data is there.

## chunk-usage, step 5 (subset): English out of core
- Core no longer holds prose: `tab_label` (the page already names its tabs from `Strings`), `TokenActivityView::label`,
  `models::plan_label`, `GROUP_LABELS`/`group_label` and `Load::message` are gone. Typed values replace them:
  `GroupName` (+ `Model::group_name`), `Note` (`Loading`/`Unavailable`/`Error(host text)` from `Load::note`), carried by
  `Left::Group`, `ChatLine::Message`, `TaskLine::Message`, `plan::Line::Notice`.
- `src/pages/usage/strings.rs` gains `plan_name`, `group_name`, `note` and their fields (oracle `SubscriptionDisplay::
  Analytics`, `GROUP_LABELS`, `Load::message`), byte for byte.
- Left as debt: the host's load errors (`Request timed out...`, report errors, `Untitled chat`) are English in the host
  as in the oracle's data layer; the normalize/tokens/plan error texts and the activity/month strings in core
  (`core/src/screens/usage/{normalize,activity,date}`) are still prose in core and want typed errors in a later step.
