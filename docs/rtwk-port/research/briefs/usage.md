# Brief: UsagePage, all reports, modes and reset credits (screen map U01-U03, tm alias `usage`, S20)

Recon 2026-11-22 (finished by recon-usage-status, read-only). Paths: `codex-rs/tui-rtwk` (ours), `codex-rs/tui/src` (oracle),
`~/.cache/codexcli/backend-pin/codex-rs/backend-client` (backend client).

## 0. First step for the porter
1. Generate the shard (no oracle shard exists today; the analytics tests are listed as "Not VS2: S20 landed",
   `tests/oracle/tools/shards.py:85`, `docs/rtwk-port/test-shards.md:130,149`). In `shards.py` add one RULES line before the OUT list
   `(r'^analytics(/|\.rs$|_tests\.rs$)', 'S20', 'routes-lists', 'usage')` and delete the OUT line `(r'^analytics', ...)`; run
   `python3 -I tests/oracle/tools/shards.py` then `--check`. Measured with the script's own `scan()`/`classify()` (2026-11-22): 158 tests
   = 34 snap + 12 ui (one shard `tests/oracle/usage.rs`, 46 stubs, under MAX 60) + 112 logic (state tests: core tests). Owner `routes-lists`
   (existing `tests/kit/verdict.rs:20`, so the kit needs no new Owner). The run also rewrites `tests/oracle.rs` (module list) and `test-shards.md`.
   Only if the person says no (section 7): skip this and carry the snaps as page tests.
2. No stale pendings: the page has no shard; `chat_live_6.rs:12-394` reset tests are all pending for a real reason (no reset flow).
3. Read `widgets/tab_strip.rs` (the shared filled tab strip, oracle `render_filled_tab_bar`) before touching the tab row: ours hand-builds it.

## 1. Scope
Oracle `analytics.rs` (536: `AnalyticsView`, keys 296-480) + `analytics/` (27 files of code, 12.4k lines with tests) + `analytics_tests.rs`:
- Chrome `render.rs` (230), `chrome.rs` (180: tab labels 11-23, per-surface scroll 26-46), `hints.rs` (111), `controls.rs` (292), `mouse.rs` (55),
  `sections.rs` (305: `Section` 11-20, routing 22-33, `visible_sections()` 160-201, `start_reports` 203-267, `range_group` 118-130, `group_label` 132-145), `styles.rs`.
- Reports: `summary.rs` + `summary_panel.rs` (337), `activity_chart.rs` (319) + `activity_chart/palette.rs`, `chart.rs` (353) + `plot.rs` (176) +
  `plot/{annotations,layout,painting}.rs`, `tokens.rs` (135), `report_data.rs` (289), `normalize.rs` (364), `tasks.rs` (248) + `task_panel.rs` (399),
  `chats.rs` (201) + `chat_panel.rs` (325), `plan.rs` (121) + `plan_panel.rs` (255), `tool_panel.rs`, `dashboard.rs` (221), `models.rs` (100), `data.rs` (210, `Load<T>`).
- Data `client.rs` (291): `Live` over `codex_backend_client::AnalyticsSession` (HTTP; identity check before and after each request; cache by
  (report, start, end); `route()` 238-265 by plan; `request_error` 267); `chats.rs:39-110` lists root threads with `thread/list` over the app-server and
  joins backend estimates (timeouts 60 s list, 25 s estimates, 90 s chats, 120 s tasks); `summary.rs:19` profile (`get_token_usage_profile`).
- Usage menu + reset credits: `chatwidget/usage.rs` (601), `chatwidget/reset_credits.rs` (79), app side `app/event_dispatch.rs:2282-2320`, `app/background_requests.rs:182,896`.
- `/usage [daily|weekly|cumulative]`: `chatwidget/slash_dispatch.rs:842-852` (summary view arg; error `Usage: /usage [daily|weekly|cumulative]`).
User sees: a full-screen dashboard (heading `Usage · Plus`, tab strip, controls row, report body, two footer rows); tabs by digit/Tab/BackTab/click;
Enter focuses a report, `z` dashboard of cards, `?` help, `R` refresh, r/g/m/a/s controls, arrows/h/l day and row, PgUp/PgDn, wheel; `/usage` with a reset
credit available opens a small menu (dashboard / redeem reset) with picker, confirmation, in-flight state.

### Tabs: ours vs the oracle's `visible_sections()`
Ours `core/src/screens/usage.rs:49-66` `Report::ALL` = Overview, Usage, Messages, Plugins, Skills, Chats, fixed (= the oracle's Consumer set without Plan).
Oracle (`sections.rs:160-201`), labels `chrome.rs:11-23` (Activity reads "Messages"; Usage reads "Tokens" for Business/Enterprise):
- Consumer, plan history enabled: Overview, Usage, Plan, Messages, Plugins, Skills, Chats (7: keys 1-7).
- Consumer: Overview, Usage, Messages, Plugins, Skills, Chats.
- Business/Enterprise with thread usage supported: Overview, Credits, Tokens, Plugins, Skills, Chats; without: the same minus Chats.
- Unknown plan: Overview only. Account not loaded: none (Esc closes).
So: `Section` (8 variants) in core and `visible_sections(account kind, plan enabled, thread_usage_supported)` as a pure fn; the selected section, the
range per `RangeGroup` (Usage/Activity/Tools), the group per section, the model filter, zero-credit groups, task metric are core state (they choose the
requests); cursor, detail, scroll per surface are widget state (R14).

## 2. Tests (in-chunk: 46 shard + 26 chat_live_6 + 112 logic = 184; pass today: 0 shard, 0 reset)
- `tests/oracle/usage.rs` (to generate) 46, by oracle file: plot 10 snap; activity_chart 4 snap + palette 1 snap; analytics_tests 5 snap + 2 ui;
  summary 2 snap; plan 3 snap + 3 ui; chat_panel 2 snap; redesign 2 snap; recovery 2 snap; dashboard 1; consumer_refresh 1; client 1
  (`live_account_identity_is_visible_and_closing_aborts_pending_loads`); navigation 2 ui; styles 2 ui; connection 1 ui; compact 1 ui; account_layout 1 ui.
  All render the whole `AnalyticsView` into a `TestBackend` of a given size (`analytics_tests.rs:39-45`), so the kit's whole-screen compare fits; fixtures
  come from `analytics/test_fixtures.rs:202-242` (`view(kind)` + `seed_reports`, END_DATE, 24-hour clock).
- Logic 112 (normalize 10, client 19, account_layout 11, data 6, navigation 7, analytics_tests 7, activity_chart 4+5, plot 4, tokens 4, account_plan 4,
  chats 3, chat_panel 5, summary 3, tool_panel 3, mouse 3, ...): carry as core tests named as the oracle's (`core/tests/usage*.rs`) or as page tests where
  they assert widget state (mouse, navigation); a test that tests a ratatui internal is n/a with a one-line reason in the parity pack.
- `tests/oracle/chat_live_6.rs:9-394` usage menu + reset 26 (21 pending, 5 n/a): in this chunk (chat-live brief C9 already calls it its own task).
  File owned by chat-live: only these functions.
- Other chunks: `chat_commands_2.rs:347,361` signed-out `/usage` notice (2 pending: kit seed is signed in) and `:375` invalid view: chat-commands.
  `chat_live_6.rs:395-450` thread billing: status-card. `chat_status_1/2` rate-limit prompts: chat-status.
- Own today: `tests/pages/usage.rs` 3 (no mouse test), `core/tests/usage.rs` 4.

## 3. What exists in ours
- Core `core/src/screens/usage.rs` (258): `Model{account{name,handle,plan}, overview: Panel<Overview>, history/credits/usage: Panel<Vec<Day{date,total}>>,
  as_of, report}`; `Panel::{Loading, Unavailable, Failed, Ready}`; `Intent::{RefreshRateLimits (no-op), Report}`; `Command/Reply/Ctx/ThreadEv` empty.
  Missing: Section/visible set, per-series day values (unit, keys, labels, other remainder), ranges/groups/filters, generation ids, chats/tasks/plan/tokens data.
- View `src/pages/usage/mod.rs` (517) + `strings.rs` (151): `Screen` via `PageRoute`; heading; hand Row of tab Texts (`mod.rs:407-421`, not `tab_strip::Tabs`);
  `Divider`; fixed control text; overview body (figures, heat map as Text rows of `□/■`, insights/top); Usage = date+total list; others = message;
  keys 1-6, Tab, Esc; `z`/`?`/`R` -> `Msg::NotYet` popup (`mod.rs:510-514`); no mouse; no per-report scroll.
- Open path: `core/src/commands/dispatch.rs:359-365,442-450` (backend auth check, view arg parsed by `TokenActivityView::parse`) -> `Target::Usage`
  (`Opens::Page`, dispatch.rs:66) -> `src/pages/session/actions.rs:136-142` emits `ScreenIntent::Usage(RefreshRateLimits)`; the bare `/usage` never opens
  the menu.
- Host: nothing. `codex-backend-client` is not in `host/Cargo.toml:16-46` (the oracle's `tui/Cargo.toml:38` has it). `AnalyticsSession::from_config(config,
  http_client_factory, runtime_auth_mode)`, `account()`, `ensure_identity()`, `request(|client| ..)` (`backend-client/src/analytics_session.rs:37,138,142,155`).
- Which enum carries what (answer to the recon question):
  - Host-only asks (no app-server RPC) are variants of core `app::Command` that `protocol::wire` maps to `None` (`core/src/protocol/mod.rs:84-85,152-153`)
    and the host runs in `host/src/in_process.rs:590-605` `work()`: `Startup{ask}`, `Import{ask}`, `Directories`, `SaveGrouping`, `CreateWorktree`. The
    analytics loads are this kind: `Command::Usage { ask: usage::Ask }` answered with `usage::Ev` under the ask's generation, host module
    `host/src/analytics.rs` (pattern `host/src/import.rs`), fake in `src/services/backend/fake.rs:448-460`.
  - Account-scoped app-server RPCs are app `Command` variants with an `AppCtx` (`core/src/app.rs:231-305,339-360`): `ReadRateLimits{ticket, backend}` +
    `AppCtx::RateLimits{backend}`. Reset credits: reads ride `ReadRateLimits` with an origin (status-card adds `RateLimitOrigin`), consume = new
    `Command::ConsumeResetCredit{ticket, backend, idempotency_key, credit_id}` (`account/rateLimitResetCredit/consume`, common.rs:1443) + `AppCtx`.
  - Not `ThreadFx`: no thread RPC is involved (ADR-003 kept); `thread/list` for Chats runs inside the host ask, as the oracle does.

## 4. Flutter -> rtwk
- Page = `Scaffold` + `TabBar` over a `TabController`; the oracle's tab look is the filled strip: ours `src/widgets/tab_strip.rs` `Tabs` (G36/G45 "not a gap",
  used by `src/pages/agents/view/mod.rs:48`, `src/sheets/select/dialog.rs:45`); tap selects (GestureDetector inside). Selected section stays in core.
- Controls row (`r 7d/30d`, `g Feature`, `m model`, `a`, `s`): `SegmentedButton`/`DropdownButton` would be Flutter; the oracle draws key-labelled text
  segments, so a `Row` of `GestureDetector(Text)` (tap = the key's intent); rtwk-controls `SegmentedButton` (`segmented_button.rs:98`) only if its caps
  look matches (it draws caps/selection marks the oracle has not: try, then Row).
- Plot (stacked daily columns, value labels, partial cells by midpoint, signed baseline, selected `▲`, date axis): Flutter = `CustomPainter` when fl_chart
  lacks it. rtwk-data `BarChart` (`rtwk-data/src/bar_chart.rs:177`: one value per bar, labels, value text) has no stacked segments, no y ticks, no
  selection mark: run it on `plot__seven_day_totals` once, then `CustomPaint` + `CellPainter` (rtwk-widgets/src/layout/custom_paint.rs; precedent G40
  "not a gap") with the bar/segment math as pure core fns from `plot/layout.rs` + `painting.rs` (data -> cells, not text layout).
- Activity heat map (`□/■` weeks x days, month labels, palette light/dark): rtwk-data `Heatmap` (`heatmap.rs:220`: `scale(HeatmapScale::shades_of(..))`,
  `row_labels`, `column_labels`, `cell_width`) first; colours from theme tokens (rule: no literals).
- Report body scroll + reading position per surface: `SingleChildScrollView` + one `ScrollController` per section / help / dashboard kept in widget
  state (PageStorage) = rtwk-scroll `SingleChildScrollView` (`single_child_scroll_view.rs`) + `ScrollController` (`scrollable.rs:703`); wheel 3 rows is the
  controller's, no hand offsets.
- Rows with a cursor (chats table, plugin/skill rows, plan periods): `ListView` + controller = `widgets/session_list.rs` or rtwk-data `SelectionController`
  (`selection_controller.rs:72`); details beside or under = `LayoutBuilder` choosing the layout (oracle `recovery` snaps: details beside the plot when wide).
  Chats table columns: rtwk-data `Table` (`table.rs`) or `Grid` content tracks.
- Dashboard `z`: `GridView` of `Card`s = `Grid` 1 or 2 columns (`LayoutBuilder`, width >= 104) of `Container` + `BoxDecoration::title(BorderTitle)`
  (rtwk-widgets/src/layout/container.rs:215) for `╭─ ▸ 1 Overview ─╮`; "… z to maximize" truncation = `dots_end` composition.
- Help `?`: a mode of the page (Amendment 2026-11-19b), its own scroll controller; not a route, not a dialog.
- Async loads: `FutureBuilder` keyed by generation = core `Load<T>` + `Ask{generation}`; `R` bumps the generation; a late reply of an old generation is
  dropped in `apply`; closing the page cancels (generation to none).
- Usage menu / reset picker / confirmation: `showModalBottomSheet` with a list = the app's select sheet (`src/sheets/select`, as the oracle uses
  `SelectionViewParams`); in-flight confirmation not dismissible (Esc ignored, Ctrl+C still quits).

## 5. Area and conflicts (tm active claims 2026-11-22)
- Porter's area (all free): `core/src/screens/usage.rs` -> `core/src/screens/usage/` (mod, model, sections, normalize, report_data, tokens, plan, chats,
  tasks, reset, seed), `core/tests/usage.rs` (+ `core/tests/usage_*.rs`), `src/pages/usage/**`, `tests/pages/usage.rs`, `tests/oracle/usage.rs` (new),
  `tests/oracle/tools/shards.py` (one rule), `docs/rtwk-port/test-shards.md` + `tests/oracle.rs` (regenerated lines only), `docs/rtwk-port/parity/usage.md`,
  `host/src/analytics.rs` (new), `host/Cargo.toml` (+ `codex-backend-client` at the backend-pin path, own commit; `Cargo.lock`), `src/sheets/usage_menu/` (new).
- Needs in files others hold (minimal changes; orchestrator sequences):
  - `core/src/app.rs` (keymap porter): `Command::Usage{ask}`, `Command::ConsumeResetCredit{..}` + its `AppCtx`, reply routing to `usage::apply`.
  - `host/src/in_process.rs` (keymap porter): one `work()` arm `Command::Usage { ask } => crate::analytics::run(..)`, one for the consume RPC via `wire`.
  - `core/src/protocol/mod.rs` (free; status-card widens to it): consume request/decode; `rate_limit_reset_credits` decode comes from status-card step 3.
  - `core/src/app/session.rs` (free; status-card widens): `RateLimitOrigin::{UsageMenu, ResetPicker, Startup}` handling (status-card adds the enum).
  - `core/src/commands/dispatch.rs:359-365` (free): bare `/usage` -> the menu when a reset credit is known available, else the page (oracle `usage.rs:17-30`).
  - `src/sheets/mod.rs` (shared): register `usage_menu`. `src/services/backend/fake.rs` (shared): fake `Command::Usage` answers.
  - `tests/oracle/chat_live_6.rs` (chat-live): functions at lines 9-394 only. `tests/oracle_snap/mod.rs` + `tests/kit/compare.rs` (kit, shared): a reader for
    the plot snaps' `Palette:` / `Rows (column:palette):` sections (`plot_tests.rs:56`), only analytics uses that format.
  - `src/pages/session/actions.rs:136-142` (settings porter): no change (the usage intent stays).
- Keymap: the oracle drives the page by `RuntimeKeymap.list` `ListAction`s (`analytics.rs:307-336`); use the list context the keymap porter builds
  (`core/src/keymap`, not landed yet); until then bind the default keys in one table in `src/pages/usage/keys.rs`.
- ADR conformance: `usage::Model` is app-scoped in `ScreenStates` (one writer, `apply`); no thread state copied; IO only in `host/src/analytics.rs`;
  widget state = cursors and scroll controllers only.

## 6. The plan (one job, tests first in each step)
1. Shard (section 0) + seeds: a JSONL seed per oracle fixture (`test_fixtures.rs` `view(Consumer|Business|Enterprise|Unknown)`, `seed_reports`, the plan,
   chats and tasks fixtures) and one debug entry per shard test state (`src/pages/usage/fixtures/`, `EntryOpen::Route`).
2. Core model (no IO): `Section`, `visible_sections`, `tab_label`, `RangeGroup`, ranges, groups, `group_label`, model filter, `Load<T>` + generation,
   `Series{unit, updated_at, days[{date, total, values[{key,label,value}]}]}`, `normalize.rs`, `report_data.rs`, `tokens.rs`, `tasks.rs`, `chats.rs`, `plan.rs`
   pure parts carried with a comment naming the oracle file; logic tests of normalize, report_data, data, tokens, tasks, chats, plan, account_plan,
   account_layout (state ones) as core tests.
3. Chrome: heading, `tab_strip::Tabs` over `visible_sections`, controls row (keys + taps), footer rows per `hints.rs` (widths < 50 / 80), loading /
   empty / error / timeout texts (`data.rs`); turns on `analytics_tests` loading/errors, `redesign__reports_keep_tabs_fixed_*`, `styles` ui 2, `compact`, `connection`.
4. Overview: `summary.rs` + `summary_panel.rs` (daily/weekly/cumulative view, `g`), activity heat map (Heatmap, palette by theme); snaps `summary__*` 3, activity_chart 4 + palette 1.
5. Plot: chart + plot engine (`CustomPaint` after the BarChart trial), selection `▲` and callout, Left/Right; snaps `plot__*` 10 (+ the palette reader),
   `analytics_tests` series/denominators/refunds snaps.
6. Reports: Credits, Tokens (business), Messages, Plugins, Skills (tool panel, details on Enter), Chats (business table + details; consumer tasks with
   `s` metric), Plan (periods, 12-hour clocks); snaps `chats_table__*` 2, `consumer_refresh__*` 1, `plan__*` 3 + ui 3, `recovery__*` 2.
7. Modes, keys, mouse: Enter focus / `z` dashboard (cards, 1-2 columns), `?` help, per-surface scroll controllers, PgUp/PgDn, Home/End, Tab/BackTab,
   digits 1-7, q / Ctrl+C / Esc; mouse tab and control taps, wheel on the body (ADR-011, `rtwk_test` mouse tests); `navigation` ui 2, `dashboard__*` 1,
   `redesign__narrow_help`, mouse/navigation logic tests as page tests.
8. Host service: `host/src/analytics.rs` (session from config, identity check around each request, cache by (report, start, end), plan routing,
   timeouts 30/60/90/120 s, `request_error` texts, `thread/list` roots + estimates for Chats, profile), `Command::Usage`, fake host answers; `R`
   refresh, stale generation dropped, close aborts; `client__live_account_identity_*` snap; client/connection/runtime_auth logic tests where they test our code.
9. Usage menu + reset credits: `src/sheets/usage_menu/` (menu, picker sorted by soonest expiry, confirmation with backend copy and one idempotency key per
   credit, in-flight not dismissible, success/failure/already-redeemed, post-consume refresh), startup reset hint; core `usage/reset.rs` state with
   request ids; unpend `chat_live_6.rs:12-394`. Needs status-card step 3 (origins, `rate_limit_reset_credits`).
10. Final: shard 46 green or `pending(Gxx)` on open rows, chat_live_6 reset 21 + 5 n/a, logic tests counted in the pack, parity pack, pty run of `/usage`
    beside the oracle (keys and mouse on the main path).

## 7. Gaps, risks, decisions
- Charts: rtwk-data `BarChart` lacks stacked segments / y ticks / selection mark; route = `CustomPaint` (G40 precedent), not a gap unless a cell cannot
  be drawn. `reuse-map.md:67,101` still says "REPLACE plots by rtwk-data, divergence for the person to approve": superseded by the 2026-11-18 no-delta
  ruling; the brief assumes cell parity.
- Two snaps join two frames with a newline (`redesign_tests.rs:19`, others): compare each half with `assert_snap_region`; the plot snaps carry a
  `sign=` prelude: compare from the row after it.
- Plan-kind data needs a real ChatGPT account for the pty run: run with the fake host fixtures and say so in the pack.
- Host dependency `codex-backend-client` (backend crate used as is) is new to `host/Cargo.toml`: one commit, named in the handover.
- Decision for the person, open as "analytics generated shard": generate it. Recommendation: yes. Evidence: one rule line in `shards.py`; 46 screen
  tests fit one shard; every snap is a whole `AnalyticsView` screen at a fixed size, the kit's native compare; without it nothing measures this page,
  the only big page with no shard. Cost: a palette reader in the kit for the 10 plot snaps.
- Decision for the orchestrator: one porter or two. Recommendation: two, in parallel. Files are disjoint (usage: `screens/usage*`, `pages/usage`,
  `sheets/usage_menu`, `host/src/analytics.rs`, shard `usage`; status-card: `screens/status*`, `cells/status`, shard `status`); they meet only in
  app-level lines (`app.rs` Command/AppCtx, `in_process.rs` arms, `protocol/mod.rs` rate-limit decode, `session.rs` refresh origin). Status-card owns
  the rate-limit plumbing (its step 3); usage step 9 lands after it; usage steps 1-7 need nothing from anyone, step 8 needs the keymap porter's
  `app.rs`/`in_process.rs` landed. One porter carrying both would serialise ~230 tests behind the running status-card job.
- Decision for the orchestrator: the 26 usage/reset functions of `chat_live_6.rs` go to this porter (chat-live brief C9 already sets them apart).
